use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::process::Command;
use string_builder::Builder;
use structs::utils::{append_to_file, get_ok_or_error, print_message};
use which::which;

pub const COPY: &str = "cp";
pub const CURL: &str = "curl";
pub const DNSMASQ: &str = "dnsmasq";
pub const FDISK: &str = "fdisk";
pub const KPARTX: &str = "kpartx";
pub const LVDISPLAY: &str = "lvdisplay";
pub const MOUNT: &str = "mount";
pub const MOVE: &str = "mv";
pub const NFS_SERVICE: &str = "nfs-kernel-server";
pub const PING: &str = "ping";
pub const PVS: &str = "pvs";
pub const RSYNC: &str = "rsync";
pub const RPCBIND: &str = "rpcbind";
pub const SCREEN: &str = "screen";
pub const SERVICE: &str = "service";
pub const SSH: &str = "ssh";
pub const UMOUNT: &str = "umount";
pub const UNZIP: &str = "unzip";
pub const VGCHANGE: &str = "vgchange";
pub const QEMU_IMG: &str = "qemu-img";
pub const ZIP: &str = "zip";

pub const BASE_DIR: &str = "/etc/cluster-manager";
pub const CONFIG: &str = "/etc/cluster-manager/config.yml";
pub const LOGS_DIR: &str = "/etc/cluster-manager/logs";
pub const OS_IMAGES_DIR: &str = "/etc/cluster-manager/os_images";
pub const TMP_DIR: &str = "/etc/cluster-manager/tmp";
pub const DNSMASQ_NODES_CONFIG_FILE: &str = "/etc/cluster-manager/nodes.dnsmasq";
pub const DNSMASQ_CONFIG_FILE: &str = "/etc/cluster-manager/dnsmasq.conf";
const DEFAULT_DNSMASQ_CONFIG_FILE: &str = "/etc/dnsmasq.conf";
pub const RESULTS_DIR: &str = "/etc/cluster-manager/results";
pub const TFTP_BASE_DIR: &str = "/tftpboot";
pub const NFS_BASE_DIR: &str = "/nfs";

pub fn get_binary_requirements() -> Vec<String> {
    let mut vec = Vec::new();
    vec.push(COPY);
    vec.push(CURL);
    vec.push(FDISK);
    vec.push(KPARTX);
    vec.push(LVDISPLAY);
    vec.push(MOUNT);
    vec.push(MOVE);
    vec.push(PING);
    vec.push(PVS);
    vec.push(RSYNC);
    vec.push(SCREEN);
    vec.push(SERVICE);
    vec.push(SSH);
    vec.push(UMOUNT);
    vec.push(UNZIP);
    vec.push(VGCHANGE);
    vec.push(QEMU_IMG);
    vec.push(ZIP);
    vec.into_iter()
        .map(std::string::ToString::to_string)
        .collect::<Vec<String>>()
}

pub fn get_service_requirements() -> Vec<String> {
    let mut vec = Vec::new();
    vec.push(DNSMASQ);
    vec.push(NFS_SERVICE);
    vec.push(RPCBIND);
    vec.into_iter()
        .map(std::string::ToString::to_string)
        .collect::<Vec<String>>()
}

pub fn check() {
    for command in get_binary_requirements() {
        println!(
            "{}",
            format!(
                "{}: checking {}",
                get_ok_or_error(which(&command).is_ok()),
                command
            )
        );
    }
    for service in get_service_requirements() {
        println!(
            "{}",
            format!(
                "{}: checking {}",
                get_ok_or_error(
                    Command::new(SERVICE)
                        .arg(&service)
                        .arg("status")
                        .output()
                        .unwrap()
                        .status
                        .success()
                ),
                service
            )
        );
    }
}

pub fn install() {
    for directory in &[
        BASE_DIR,
        OS_IMAGES_DIR,
        TMP_DIR,
        NFS_BASE_DIR,
        LOGS_DIR,
        TFTP_BASE_DIR,
        RESULTS_DIR,
    ] {
        let directory_path = Path::new(directory);
        print_message(
            &format!("creating {}", directory_path.as_os_str().to_str().unwrap()),
            directory_path.exists() || fs::create_dir_all(directory_path.as_os_str()).is_ok(),
        );
    }
    fs::set_permissions(OS_IMAGES_DIR, fs::Permissions::from_mode(0o777)).unwrap();
    let mut needs_restart = false;
    let config = Path::new(CONFIG);
    if !config.exists() {
        fs::write(config, "").unwrap();
    }
    let dnsmasq_config = Path::new(DNSMASQ_CONFIG_FILE);
    if !dnsmasq_config.exists() {
        needs_restart = true;
        fs::write(DNSMASQ_CONFIG_FILE, default_dnsmasq_conf_content()).unwrap();
        append_to_file(
            DEFAULT_DNSMASQ_CONFIG_FILE,
            format!("conf-file={}", DNSMASQ_CONFIG_FILE),
        )
        .unwrap();
    }
    let dnsmasq_nodes_config = Path::new(DNSMASQ_NODES_CONFIG_FILE);
    if !dnsmasq_nodes_config.exists() {
        needs_restart = true;
        fs::write(dnsmasq_nodes_config, "").unwrap();
    }
    if needs_restart {
        Command::new(SERVICE)
            .arg(DNSMASQ)
            .arg("restart")
            .spawn()
            .unwrap()
            .wait_with_output()
            .unwrap();
    }
}

fn default_dnsmasq_conf_content() -> String {
    let mut builder = Builder::default();
    builder.append("log-dhcp\n");
    builder.append("enable-tftp\n");
    builder.append("tftp-root=/tftpboot\n");
    builder.append("pxe-service=0,\"Raspberry Pi Boot\"\n");
    builder.append("log-facility=/var/log/dnsmasq.log\n");
    builder.append("local=/cluster/\n");
    builder.append("domain=cluster\n");
    builder.append("conf-file=/etc/cluster-manager/nodes.dnsmasq\n");
    builder.string().unwrap()
}
