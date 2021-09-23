use crate::config::{
    get, get_default_os_for, get_log_sources_of, get_node_by_id, get_storage_device_of,
};
use crate::database::{get_idle_nodes, insert_deployment, insert_service, insert_task};
use crate::installer::{
    BASE_DIR, COPY, FDISK, KPARTX, LVDISPLAY, MOUNT, NFS_BASE_DIR, OS_IMAGES_DIR, PVS, QEMU_IMG,
    RESULTS_DIR, RSYNC, SCREEN, SSH, TFTP_BASE_DIR, TMP_DIR, UMOUNT, UNZIP, VGCHANGE,
};
use crate::node_manager::{
    change_hostname, change_ipv4address, get_ipv4_address, remove_nfsroot, restart_services,
    umount_tftp_directory,
};
use crate::power_manager::reboot;
use chrono::Utc;
use colored::Colorize;
use core::time;
use std::path::Path;
use std::process::Command;
use std::str;
use std::{fs, io, thread};
use structs::architecture::Architecture::X86;
use structs::bootconfig::{group, BootConfig};
use structs::configuration::Configuration;
use structs::deployment::Deployment;
use structs::logsource::LogSourceTypes;
use structs::mountpoint::Mountpoint;
use structs::node::Node;
use structs::partition::Partition;
use structs::service::Service;
use structs::task::Task;
use structs::task::Type::GetResults;
use structs::utils::{get_random_name, print_information, print_message, replace_in_file};
use yaml_rust::YamlLoader;
use crate::logs_manager::gather_logs;

const BUILD_DIRECTORY: &str = "os-build";

pub fn deploy_deployment(deployment_yaml: &str) -> bool {
    let path = Path::new(deployment_yaml);
    if !path.exists() {
        println!("{}", "please provide an existing yaml file".red());
        return false;
    }
    let content =
        &fs::read_to_string(deployment_yaml).expect("Something went wrong reading the file");
    let yaml = YamlLoader::load_from_str(content).unwrap();
    let mut deployment = Deployment::from_yaml(deployment_yaml, &yaml[0]);
    deploy(&mut deployment)
}

pub fn deploy(deployment: &mut Deployment) -> bool {
    print_message(
        "check image architectures",
        associate_architectures(deployment),
    );
    if let Ok(services_with_nodes) = check_availability(deployment.get_services()) {
        let id = insert_deployment(&deployment).unwrap();
        deployment.id = Some(id);
        print_message("add deployment to database", true);
        let mut nodes = Vec::new();
        for (mut service, mut node) in services_with_nodes {
            service.deployment = Some(id);
            deploy_service(deployment, &mut service, &mut node);
            nodes.push(node);
        }
        restart_services();
        for node in nodes {
            print_message(&format!("rebooting node {}", node.id), reboot_node(&node));
        }
    } else {
        println!(
            "{}",
            "can not deploy this deployment because not enough nodes are available".red()
        );
        return false;
    }
    true
}

pub fn deploy_single_image(image: &str, node: Option<Node>) -> bool {
    let mut deployment = Deployment::new(get_random_name().as_str());
    let mut service = Service::new(
        get_random_name().as_str(),
        image,
        get_random_name().as_str(),
    );
    service.preferred_node = match node {
        Some(n) => Some(n.id),
        None => None,
    };
    deployment.services.push(service);
    deploy(&mut deployment)
}

fn deploy_service(deployment: &mut Deployment, service: &mut Service, node: &mut Node) -> bool {
    let status = deploy_image(deployment, service, &service.image, &node);
    print_message(
        format!("deploying service {} on {}", service.name, node.id).as_str(),
        status,
    );
    if status {
        change_hostname(node, service.hostname.as_str());
        if let Some(ipv4) = &service.ipv4_address {
            change_ipv4address(node, ipv4);
        } else {
            service.ipv4_address = get_ipv4_address(&node);
        }
        print_message("create results directory", create_results_directory(&node));
        print_information("starting logging from serial inputs");
        open_screens_for_serial_logging(&node);
        let service_id = insert_service(&service);
        print_message("add service in database", service_id.is_ok());
        if let Some(configuration) = extract_configuration(&service.image) {
            if configuration.on_device {
                let mountpoint = configuration
                    .mountorder
                    .iter()
                    .filter_map(|p| {
                        if p.get_path().eq("/") {
                            Some(p.to_owned())
                        } else {
                            None
                        }
                    })
                    .collect::<Vec<Mountpoint>>();
                service.id = Some(service_id.unwrap());
                if !mountpoint.is_empty() {
                    let task: Task = Task::new(
                        Some(deployment.clone()),
                        Some(service.clone()),
                        GetResults,
                        serde_json::to_string(&mountpoint.get(0).unwrap()).unwrap(),
                        false,
                    );
                    insert_task(&task, deployment.id.unwrap()).unwrap();
                    deployment.tasks.push(task);
                }
            }
        }
    }
    status
}

pub fn check_availability(services: Vec<Service>) -> Result<Vec<(Service, Node)>, String> {
    let mut services_with_nodes = Vec::new();
    let mut available_nodes = get_idle_nodes().unwrap();
    for mut service in services {
        for _replica in 0..service.replicas {
            let mut to_remove = None;
            for (index, node) in available_nodes.iter().enumerate() {
                let arch = service.architecture.as_ref().unwrap();
                let mut matched = false;
                match &service.preferred_node {
                    Some(preferred_node) => {
                        if node.id.eq(preferred_node) {
                            matched = true;
                        }
                    }
                    None => {
                        matched = true;
                    }
                }
                if matched && node.architecture.eq(arch) {
                    to_remove = Some((index, node.clone()));
                    service.node = Some(node.id.clone());
                    break;
                }
            }
            match to_remove {
                Some((index, node)) => {
                    available_nodes.remove(index);
                    services_with_nodes.push((service.clone(), node));
                }
                None => {
                    return Err(format!("No available node for {}", service.name.as_str()));
                }
            }
        }
    }
    Ok(services_with_nodes)
}

#[allow(dead_code)]
fn choose_node_for_service(service: &Service) -> Option<Node> {
    let first_choice = match &service.preferred_node {
        Some(mac) => match get_node_by_id(mac, false) {
            Some(node) => {
                if service.replicas == 1 {
                    Some(node)
                } else {
                    None
                }
            }
            None => None,
        },
        None => None,
    };
    let available_nodes = get_idle_nodes().unwrap();
    if let Some(node) = first_choice {
        if available_nodes.contains(&node) {
            return Some(node);
        }
    }
    match available_nodes.first() {
        Some(node) => Some(node.clone()),
        None => None,
    }
}

pub fn deploy_image(
    deployment: &mut Deployment,
    service: &Service,
    image_path: &str,
    node: &Node,
) -> bool {
    let path_string = format!("{}/{}.zip", OS_IMAGES_DIR, image_path);
    let path = Path::new(&path_string);
    if !path.exists() {
        println!("{}", "please provide a valid image_path".red());
        return false;
    }
    let sandbox_name = structs::utils::get_random_name();
    print_information(&format!(
        "chosen name for sandbox is {}",
        sandbox_name.green()
    ));
    print_information(&format!("chosen node is {}", node.id.green()));
    print_message(
        "create deploy sandbox",
        create_tmp_directory(&sandbox_name).is_ok(),
    );
    print_message(
        "unpack image in sandbox",
        unpack_image(&path_string, &sandbox_name),
    );
    let configuration = read_configuration(&sandbox_name);
    print_message("read configuration", configuration.is_some());
    let config = configuration.unwrap();
    if config.on_device {
        deploy_image_on_local_storage(deployment, service, &sandbox_name, node);
        if node.pxe {
            print_message("write pxefile", write_pxe_file(&config, &node));
        }
    } else {
        deploy_image_for_netboot(&sandbox_name, config, node);
    }
    print_message(
        "destroy deploy sandbox",
        destroy_tmp_directory(sandbox_name.as_str()).is_ok(),
    );
    true
}

fn deploy_image_on_local_storage(
    deployment: &mut Deployment,
    service: &Service,
    sandbox_name: &str,
    node: &Node,
) -> bool {
    print_message("deploying image to be written on local storage", true);
    if let Some(default_os) = get_default_os_for(node) {
        if deploy_image(deployment, service, &default_os, node) {
            print_message("rebooting node", reboot_node(node));
            print_message(
                "copying image to node",
                move_image_to_root_home(sandbox_name, node),
            );
            print_message(
                "allow ssh key to connect to node",
                allow_ssh_access_to_node(node),
            );
            wait_for_rebooted_node(node);
            let storage_device = get_storage_device_of(node);
            print_message(
                "flashing image to node",
                flash_image_to_node(node, &format!("/dev/{}", storage_device.unwrap())),
            );
            print_message("reboot via ssh", !execute_command_over_ssh(node, "reboot"));
            print_message(
                "unmount tftpboot directory",
                umount_tftp_directory(&node.tftp_prefix),
            );
            print_message(
                "remove filesystem",
                remove_nfsroot(node.tftp_prefix.as_str(), false).is_ok(),
            );
        }
        return true;
    }
    false
}

fn deploy_image_for_netboot(sandbox_name: &str, mut config: Configuration, node: &Node) {
    if config.architecture.get_name().eq(X86.get_name()) {
        print_message("convert vmdk to img", convert_vmdk_to_img(sandbox_name));
    }
    let loopdevice = get_loopdevice(sandbox_name);
    print_message("add new loopdevice", loopdevice.is_ok());
    let mapper = loopdevice.unwrap();
    print_information(&format!("loopdevice is: {}", mapper));
    let mut vg = String::new();
    if is_lvm(sandbox_name) {
        let vg_ret = Some(handle_lvm_image(&config, &mapper, sandbox_name)).unwrap();
        vg = vg_ret.0;
        config.partitions = vg_ret.1;
        for (i, partition) in config.partitions.iter().enumerate() {
            print_message(
                format!("mount lvm-partition #{}", i + 1).as_str(),
                mount_lvm_partition(sandbox_name, &vg, i + 1, &partition),
            );
        }
    } else {
        if config.mountorder.is_empty() {
            print_information("no mountorder in configuration, falling back on partitions");
            for (i, partition) in config.partitions.iter().enumerate() {
                config
                    .mountorder
                    .push(Mountpoint::new(i + 1, i + 1, partition.get_mountpoint()));
            }
        }
        for partition in &config.mountorder {
            print_message(
                format!("mount partition #{}", partition.partition_number).as_str(),
                mount_partition(sandbox_name, mapper.as_str(), &partition),
            );
        }
    }
    print_message(
        "create result directory",
        create_result_directory(sandbox_name).is_ok(),
    );
    if is_lvm(sandbox_name) {
        for (i, p) in config.partitions.iter().enumerate() {
            let partition = Mountpoint::new(i + 1, i + 1, p.get_mountpoint());
            print_message(
                format!("copy partition #{} to result", partition.partition_number).as_str(),
                copy_partition_to_result(&partition, sandbox_name),
            );
            print_message(
                format!("umount partition #{}", partition.partition_number).as_str(),
                unmount_partition(&partition, sandbox_name),
            );
            print_message(
                format!("remove partition #{} directory", partition.partition_number).as_str(),
                remove_partition_directory(&partition, sandbox_name).is_ok(),
            );
        }
        deactivate_vgs(&vg);
    } else {
        for partition in &config.mountorder {
            print_message(
                format!("copy partition #{} to result", partition.partition_number).as_str(),
                copy_partition_to_result(&partition, sandbox_name),
            );
            print_message(
                format!("umount partition #{}", partition.partition_number).as_str(),
                unmount_partition(&partition, sandbox_name),
            );
            print_message(
                format!("remove partition #{} directory", partition.partition_number).as_str(),
                remove_partition_directory(&partition, sandbox_name).is_ok(),
            );
        }
    }
    print_message("remove loopdevice", remove_loopdevice(sandbox_name));
    print_message(
        "resolve bootconfigs",
        resolve_bootconfigs(
            sandbox_name,
            &group(&config.bootconfigs),
            node.tftp_prefix.as_str(),
            true,
        ),
    );
    if config.pxe {
        print_message("write pxefile", write_pxe_file(&config, &node));
    }
    print_message(
        "unmount old tftpboot directory",
        umount_tftp_directory(node.tftp_prefix.as_str()),
    );
    print_message(
        "copy image result to nfsroot",
        move_result_to_nfs(sandbox_name, node.tftp_prefix.as_str()),
    );
    print_message(
        "mount boot partition in tftpboot",
        mount_tftpboot(config.partitions, node.tftp_prefix.as_str()),
    );
}

fn create_tmp_directory(directory: &str) -> io::Result<()> {
    fs::create_dir(format!("{}/{}", TMP_DIR, directory))
}

fn destroy_tmp_directory(directory: &str) -> io::Result<()> {
    fs::remove_dir_all(format!("{}/{}", TMP_DIR, directory))
}

fn unpack_image(image_path: &str, directory: &str) -> bool {
    let complete_output = Command::new(UNZIP)
        .arg(image_path)
        .arg("-d")
        .arg(format!("{}/{}", TMP_DIR, directory))
        .output()
        .expect("failed to unpack image");
    complete_output.status.success()
}

fn get_loopdevice(directory: &str) -> Result<String, ()> {
    let complete_output = Command::new(KPARTX)
        .arg("-av")
        .arg(format!(
            "{}/{}/{}/generated.img",
            TMP_DIR, directory, BUILD_DIRECTORY
        ))
        .output()
        .expect("failed to unpack image");
    let stdout = str::from_utf8(&complete_output.stdout).unwrap();
    if !complete_output.status.success() {
        return Err(());
    }
    let words = stdout
        .lines()
        .next()
        .unwrap()
        .split_whitespace()
        .collect::<Vec<&str>>();
    if words.len() < 3 {
        return Err(());
    }
    let mut loopdevice = words.get(2).unwrap().split('p').collect::<Vec<&str>>();
    Ok(loopdevice.drain(0..2).collect::<Vec<&str>>().join("p"))
}

fn remove_loopdevice(directory: &str) -> bool {
    let complete_output = Command::new(KPARTX)
        .arg("-d")
        .arg(format!(
            "{}/{}/{}/generated.img",
            TMP_DIR, directory, BUILD_DIRECTORY
        ))
        .output()
        .expect("failed to umount image");
    complete_output.status.success()
}

fn mount_partition(directory: &str, loopdevice: &str, mountpoint: &Mountpoint) -> bool {
    let formatted = format!(
        "{}/{}/{}",
        TMP_DIR,
        directory,
        mountpoint.partition_number.to_string()
    );
    let path = formatted.as_str();
    if fs::create_dir_all(path.to_string()).is_ok() {
        let complete_output = Command::new(MOUNT)
            .arg(format!(
                "/dev/mapper/{}p{}",
                loopdevice, mountpoint.partition_number
            ))
            .arg(path.to_string())
            .output()
            .expect("failed to mount partition");
        return complete_output.status.success();
    }
    false
}

fn unmount_partition(mountpoint: &Mountpoint, directory: &str) -> bool {
    let complete_output = Command::new(UMOUNT)
        .arg(format!(
            "{}/{}/{}",
            TMP_DIR,
            directory,
            mountpoint.partition_number.to_string()
        ))
        .output()
        .expect("failed to umount partition");
    complete_output.status.success()
}

fn remove_partition_directory(mountpoint: &Mountpoint, directory: &str) -> io::Result<()> {
    let formatted = format!("{}/{}", directory, mountpoint.partition_number.to_string());
    fs::remove_dir_all(format!("{}/{}", TMP_DIR, formatted.as_str()))
}

fn create_result_directory(name: &str) -> io::Result<()> {
    fs::create_dir(format!("{}/{}/result", TMP_DIR, name))
}

fn copy_partition_to_result(mountpoint: &Mountpoint, directory: &str) -> bool {
    let mut child = Command::new(RSYNC)
        .arg("-a")
        .arg(format!(
            "{}/{}/{}/",
            TMP_DIR,
            directory,
            mountpoint.partition_number.to_string()
        ))
        .arg(format!(
            "{}/{}/result{}",
            TMP_DIR,
            directory,
            mountpoint.get_path()
        ))
        .spawn()
        .expect("failed to copy partition to result");
    child.wait().unwrap().success()
}

fn resolve_bootconfigs(
    directory: &str,
    bootconfigs: &BootConfig,
    node: &str,
    verbose: bool,
) -> bool {
    let mut success = false;
    if !bootconfigs.get_files().is_empty() {
        success = true;
        let mut placeholders: Vec<(&str, &str)> = Vec::new();
        let server_ip = get("server-ip").unwrap();
        let log_server = get("log-server").unwrap();
        placeholders.push(("%SERVER_IP%", server_ip.as_str()));
        let nfs_root = format!("{}/{}", NFS_BASE_DIR, node);
        placeholders.push(("%NFS_ROOT%", nfs_root.as_str()));
        placeholders.push(("%LOG_SERVER%", log_server.as_str()));
        for config in bootconfigs.get_files() {
            let result = replace_placeholders(
                format!("{}/{}/result{}", TMP_DIR, directory, config).as_str(),
                &placeholders,
            );
            if !result {
                success = false
            }
            if verbose {
                print_message(format!("resolving {}", config).as_str(), result);
            }
        }
    }
    success
}

pub fn replace_placeholders(file: &str, placeholders: &[(&str, &str)]) -> bool {
    let mut success = true;
    for (key, value) in placeholders {
        if replace_in_file(file, key, value).is_err() {
            success = false;
        }
    }
    success
}

fn mount_tftpboot(partitions: Vec<Partition>, node: &str) -> bool {
    let boot_path = match partitions
        .into_iter()
        .filter(|p| p.get_name().eq("boot"))
        .collect::<Vec<Partition>>()
        .first()
    {
        Some(boot) => boot.get_mountpoint(),
        None => "/boot".to_string(),
    };
    let mut child = Command::new(MOUNT)
        .arg("-o")
        .arg("bind")
        .arg(format!("{}/{}{}", NFS_BASE_DIR, node, boot_path).replace("//", "/"))
        .arg(format!("{}/{}", TFTP_BASE_DIR, node))
        .spawn()
        .expect("failed to mount nfsroot to tftpboot");
    child.wait().unwrap().success()
}

fn move_result_to_nfs(directory: &str, node: &str) -> bool {
    let mut child = Command::new(RSYNC)
        .arg("--delete-before")
        .arg("--remove-source-files")
        .arg("-a")
        .arg(format!("{}/{}/result/", TMP_DIR, directory))
        .arg(format!("{}/{}", NFS_BASE_DIR, node))
        .spawn()
        .expect("failed to copy result to nfsroot");
    child.wait().unwrap().success()
}

fn execute_command_over_ssh(node: &Node, command: &str) -> bool {
    Command::new(SSH)
        .arg("-o")
        .arg("StrictHostKeyChecking=no")
        .arg("-o")
        .arg("UserKnownHostsFile=/dev/null")
        .arg("-i")
        .arg(format!("{}/deployer", BASE_DIR))
        .arg(format!("root@{}", node.ipv4_address))
        .arg(command)
        .spawn()
        .expect("failed to execute command over ssh")
        .wait()
        .unwrap()
        .success()
}

fn flash_image_to_node(node: &Node, device: &str) -> bool {
    execute_command_over_ssh(
        node,
        &format!(
            "dd if=/root/generated.img of={} bs={} status=progress",
            device, "20M"
        ),
    )
}

fn allow_ssh_access_to_node(node: &Node) -> bool {
    Command::new(COPY)
        .arg(format!("{}/deployer.pub", BASE_DIR))
        .arg(format!(
            "{}/{}/root/.ssh/authorized_keys",
            NFS_BASE_DIR, node.tftp_prefix
        ))
        .spawn()
        .expect("failed to copy ssh key to node")
        .wait()
        .unwrap()
        .success()
}

fn move_image_to_root_home(directory: &str, node: &Node) -> bool {
    let image_command = Command::new("ls")
        .arg("--sort=size")
        .arg(format!("{}/{}/{}/", TMP_DIR, directory, BUILD_DIRECTORY))
        .output()
        .expect("ls did not work");
    let stdout = str::from_utf8(&image_command.stdout).unwrap();
    if !image_command.status.success() {
        println!("could not traverse directory");
    }
    let files = stdout
        .lines()
        .next()
        .unwrap()
        .split_whitespace()
        .collect::<Vec<&str>>();
    let image = files.get(0).unwrap();
    Command::new(COPY)
        .arg(format!(
            "{}/{}/{}/{}",
            TMP_DIR, directory, BUILD_DIRECTORY, image
        ))
        .arg(format!(
            "{}/{}/root/generated.img",
            NFS_BASE_DIR, node.tftp_prefix
        ))
        .spawn()
        .expect("failed to copy image to node")
        .wait()
        .unwrap()
        .success()
}

fn wait_for_rebooted_node(node: &Node) {
    println!("waiting for node to reboot");
    let utc = Utc::now();
    thread::sleep(time::Duration::from_millis(1000));
    loop {
        if crate::node_manager::is_up(&node.ipv4_address) {
            println!(
                "rebooting took {} seconds",
                Utc::now().signed_duration_since(utc).num_seconds()
            );
            loop {
                if execute_command_over_ssh(node, "echo 'waiting'") {
                    break;
                }
            }
            break;
        }
    }
}

fn reboot_node(node: &Node) -> bool {
    reboot(node)
}

fn prune_sd_card(node: &Node) -> bool {
    execute_command_over_ssh(
        node,
        "dd if=/dev/urandom of=/dev/mmcblk0 bs=20M status=progress",
    )
}

pub fn clean_node(node: &Node) -> bool {
    print_message(
        "deploying raspbian in order to wipe all storage on sd_card",
        true,
    );
    let mut deployment = Deployment::new("cleaning");
    let service = Service::new("cleaning", "raspbian", "cleaning");
    if deploy_image(&mut deployment, &service, "raspbian", node) {
        print_message("rebooting node", reboot_node(node));
        print_message(
            "allow ssh key to connect to node",
            allow_ssh_access_to_node(node),
        );
        wait_for_rebooted_node(node);
        print_message("prune sd_card", prune_sd_card(node));
        print_message("reboot via ssh", execute_command_over_ssh(node, "reboot"));
        print_message(
            "unmount tftpboot directory",
            umount_tftp_directory(&node.tftp_prefix),
        );
    }
    true
}

fn create_results_directory(node: &Node) -> bool {
    let path = format!("{}/{}/logs", RESULTS_DIR, node.id);
    if Path::new(path.as_str()).exists() {
        fs::remove_dir_all(&path).unwrap();
    }
    fs::create_dir_all(path).is_ok()
}

fn open_screens_for_serial_logging(node: &Node) {
    for (index, serial_device) in get_log_sources_of(&node)
        .iter()
        .filter(|l| l.source.eq(&LogSourceTypes::SERIAL))
        .enumerate()
    {
        //screen -dmS rpi2 -L -Logfile /home/pi/ttyusb.log /dev/ttyUSB0 115200
        let name = format!("{}-{}", &node.id, index);
        let status = Command::new(SCREEN)
            .arg("-dmS")
            .arg(&name)
            .arg("-L")
            .arg("-Logfile")
            .arg(format!(
                "{}/{}/logs/serial{}.log",
                RESULTS_DIR, node.id, index
            ))
            .arg(format!("/dev/{}", serial_device.path))
            .arg("115200")
            .spawn()
            .expect("failed to start screen")
            .wait()
            .unwrap()
            .success();
        print_message(&format!("starting screen {}", &name), status);
    }
}

fn read_configuration(directory: &str) -> Option<Configuration> {
    let content = fs::read_to_string(format!(
        "{}/{}/{}/configuration.json",
        TMP_DIR, directory, BUILD_DIRECTORY
    ))
    .expect("Something went wrong reading the configuration.json");
    if let Ok(configuration) = serde_json::from_str::<Configuration>(content.as_str()) {
        return Some(configuration);
    }
    None
}

fn handle_lvm_image(
    config: &Configuration,
    loopdevice: &str,
    _directory: &str,
) -> (String, Vec<Partition>) {
    let (pv, vg) = get_pv_and_vg(loopdevice);
    print_information(&format!("pv is {}", pv));
    print_information(&format!("vg is {}", vg));
    let mut partitions = Vec::new();
    for (i, lvm_partition) in lvm_partitions(&vg).iter().enumerate() {
        let parsed_name = lvm_partition
            .split(&format!("{}/", vg))
            .collect::<Vec<&str>>();
        let name = parsed_name.last().unwrap();
        if let Some(partition) = config
            .partitions
            .iter()
            .filter(|m| m.get_name().eq(name))
            .last()
        {
            partitions.push(partition.clone());
            print_information(&format!(
                "partition#{} is {} with name {}",
                i, lvm_partition, name
            ));
        }
    }
    print_message("activate vgs", activate_vgs(&vg));
    (vg, partitions)
}

fn lvm_partitions(vg: &str) -> Vec<String> {
    let child = Command::new(LVDISPLAY)
        .arg(vg)
        .output()
        .expect("could not execute lvdisplay");
    let stdout = str::from_utf8(&child.stdout).unwrap();
    let lines = stdout
        .lines()
        .filter(|l| l.contains("LV Path"))
        .collect::<Vec<&str>>();
    let mut vec = Vec::new();
    for line in lines {
        vec.push(String::from(
            *line
                .split_whitespace()
                .collect::<Vec<&str>>()
                .last()
                .unwrap(),
        ))
    }
    vec
}

fn get_pv_and_vg(loopdevice: &str) -> (String, String) {
    let child = Command::new(PVS).output().expect("could not execute pvs");
    let stdout = str::from_utf8(&child.stdout).unwrap();
    let line = stdout
        .lines()
        .filter(|l| l.contains(loopdevice))
        .last()
        .unwrap()
        .split_whitespace()
        .collect::<Vec<&str>>();
    (
        String::from(*line.get(0).unwrap()),
        String::from(*line.get(1).unwrap()),
    )
}

fn activate_vgs(vg: &str) -> bool {
    let child = Command::new(VGCHANGE)
        .arg("-ay")
        .arg(vg)
        .output()
        .expect("could not execute vgchange");
    child.status.success()
}

fn deactivate_vgs(vg: &str) -> bool {
    Command::new(VGCHANGE)
        .arg("-an")
        .arg(vg)
        .output()
        .expect("could not execute vgchange");
    let child1 = Command::new("vgexport")
        .arg(vg)
        .output()
        .expect("could not execute vgexport");
    child1.status.success()
}

fn mount_lvm_partition(
    directory: &str,
    vgname: &str,
    number: usize,
    partition: &Partition,
) -> bool {
    let formatted = format!("{}/{}/{}", TMP_DIR, directory, number.to_string());
    let path = formatted.as_str();
    if fs::create_dir_all(path.to_string()).is_ok() {
        let complete_output = Command::new(MOUNT)
            .arg(format!("/dev/{}/{}", vgname, partition.get_name()).replace("//", "/"))
            .arg(path.to_string())
            .output()
            .expect("failed to mount partition");
        return complete_output.status.success();
    }
    false
}

fn is_lvm(directory: &str) -> bool {
    let child = Command::new(FDISK)
        .arg("-l")
        .arg(format!(
            "{}/{}/{}/generated.img",
            TMP_DIR, directory, BUILD_DIRECTORY
        ))
        .output()
        .expect("could not execute fdisk");
    let stdout = str::from_utf8(&child.stdout).unwrap();
    stdout.lines().any(|l| l.contains("Linux LVM"))
}

fn convert_vmdk_to_img(directory: &str) -> bool {
    Command::new(QEMU_IMG)
        .arg("convert")
        .arg("-f")
        .arg("vmdk")
        .arg(format!(
            "{}/{}/{}/generated.vmdk",
            TMP_DIR, directory, BUILD_DIRECTORY
        ))
        .arg(format!(
            "{}/{}/{}/generated.img",
            TMP_DIR, directory, BUILD_DIRECTORY
        ))
        .output()
        .expect("could not execute qemu-img")
        .status
        .success()
}

fn write_pxe_file(config: &Configuration, node: &Node) -> bool {
    let pxe_dir = &format!("{}/pxelinux.cfg", TFTP_BASE_DIR);
    let path = Path::new(pxe_dir);
    if !path.exists() {
        fs::create_dir(path.as_os_str()).unwrap();
    }
    let default_pxe_file = format!(
        "{}/pxelinux.cfg/01-{}",
        TFTP_BASE_DIR,
        node.mac_address.replace(":", "-")
    );
    if config.on_device {
        fs::write(&default_pxe_file, "DEFAULT local\nlabel local\nLOCALBOOT 0").is_ok()
    } else {
        let mut placeholders: Vec<(&str, &str)> = Vec::new();
        let tftp_prefix = format!("{}/{}", TFTP_BASE_DIR, node.tftp_prefix);
        let server_ip = get("server-ip").unwrap();
        let nfs_root = format!("{}/{}", NFS_BASE_DIR, node.id);
        placeholders.push(("%SERVER_IP%", &server_ip));
        placeholders.push(("%NFS_ROOT%", &nfs_root));
        placeholders.push(("%TFTP_ROOT%", &tftp_prefix));
        fs::write(
            &default_pxe_file,
            format!(
                "DEFAULT {}/{} {}",
                tftp_prefix, config.pxe_kernel, config.pxe_options
            ),
        )
        .unwrap();
        replace_placeholders(&default_pxe_file, &placeholders)
    }
}

pub fn extract_configuration(image: &str) -> Option<Configuration> {
    let complete_output = Command::new(UNZIP)
        .arg("-p")
        .arg(format!("{}/{}.zip", OS_IMAGES_DIR, image))
        .arg(format!("{}/configuration.json", BUILD_DIRECTORY))
        .output()
        .expect("failed to unpack image");
    let content = str::from_utf8(&complete_output.stdout).unwrap();
    if let Ok(configuration) = serde_json::from_str::<Configuration>(content) {
        return Some(configuration);
    }
    None
}

fn associate_architectures(deployment: &mut Deployment) -> bool {
    let mut done = true;
    for service in &mut deployment.services {
        if service.architecture.is_none() {
            if let Some(image_config) = extract_configuration(&service.image) {
                service.architecture = Some(image_config.architecture)
            } else {
                done = false;
            }
        }
    }
    done
}

pub fn retrieve_local_logs(
    deployment: &mut Deployment,
    service: &Service,
    node: &Node,
    mountpoint: &Mountpoint,
) -> bool {
    print_message("deploying image to be written on local storage", true);
    if let Some(default_os) = get_default_os_for(node) {
        if deploy_image(deployment, service, &default_os, node) {
            print_message("rebooting node", reboot_node(node));
            print_message(
                "allow ssh key to connect to node",
                allow_ssh_access_to_node(node),
            );
            wait_for_rebooted_node(node);
            let storage_device = get_storage_device_of(node).unwrap();
            print_message(
                "create mountdirectory on device",
                execute_command_over_ssh(node, "mkdir /local"),
            );
            print_message(
                "mount rootsystem of device",
                execute_command_over_ssh(
                    node,
                    &format!(
                        "mount /dev/{}p{} /local",
                        storage_device, mountpoint.partition_number
                    ),
                ),
            );
            print_message(
                "move /local/results to /results via ssh",
                execute_command_over_ssh(node, "mv /local/results /results"),
            );
            gather_logs(node);
            print_message("reboot via ssh", !execute_command_over_ssh(node, "reboot"));
            print_message(
                "unmount tftpboot directory",
                umount_tftp_directory(&node.tftp_prefix),
            );
            print_message(
                "remove filesystem",
                remove_nfsroot(node.tftp_prefix.as_str(), false).is_ok(),
            );
        }
        return true;
    }
    false
}
