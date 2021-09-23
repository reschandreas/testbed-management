use std::collections::HashMap;
use std::fs;
use string_builder::Builder;
use structs::architecture::Architecture;
use structs::architecture::Architecture::{ARM32, ARM64, X86};
use structs::arm_preamble::ArmPreamble;
use structs::bootconfig::BootConfig;
use structs::imagefile::Imagefile;
use structs::partition::Partition;
use structs::preamble::Preamble;
use structs::provisioner::Provisioner;
use structs::provisioner::Types::{FILE, SHELL};
use structs::x86_preamble::X86Preamble;

pub fn parse(filename: &str) -> Option<Imagefile> {
    let content = fs::read_to_string(filename).expect("Something went wrong reading the file");
    let mut commands = sanitize_content(&content);
    if let Some(architecture) = get_architecture(&mut commands) {
        commands = commands
            .into_iter()
            .filter(|(command, _)| !command.eq("ARCH"))
            .collect::<Vec<(String, String)>>();
        let mut image = Imagefile::new(String::from("generated.img"), &architecture);
        let parsers = common_parsers();
        let arm_parsers = arm_parsers();
        let x86_parsers = x86_parsers();
        let mut arm_preamble = ArmPreamble::default();
        let mut x86_preamble = X86Preamble::default();
        for (command, args) in &commands {
            if parsers.contains_key(command) {
                parsers.get(command).unwrap()(&mut image, args.as_str());
            }
            match image.architecture {
                ARM32 | ARM64 => {
                    if arm_parsers.contains_key(command) {
                        arm_parsers.get(command).unwrap()(&mut arm_preamble, args.as_str());
                    }
                }
                X86 => {
                    if x86_parsers.contains_key(command) {
                        x86_parsers.get(command).unwrap()(&mut x86_preamble, args.as_str());
                    }
                }
            }
        }
        match image.architecture {
            ARM32 | ARM64 => {
                image.preamble = Box::new(arm_preamble);
            }
            X86 => {
                image.preamble = Box::new(x86_preamble);
            }
        }
        Some(image)
    } else {
        eprintln!("Please specify an architecture");
        None
    }
}

fn sanitize_content(content: &str) -> Vec<(String, String)> {
    let mut builder = Builder::default();
    for c in content.chars() {
        builder.append(c);
    }
    let no_fake_newlines = builder.string().unwrap();
    let stat = no_fake_newlines.replace("\\\n", "");
    let raw_lines = stat
        .lines()
        .filter(|s| !s.is_empty())
        .collect::<Vec<&str>>();
    let mut commands: Vec<(String, String)> = Vec::new();
    for line in raw_lines {
        if line.starts_with('#') {
            continue;
        }
        if let Some(command) = get_command(line) {
            commands.push(command)
        }
    }
    commands
}

fn get_command(line: &str) -> Option<(String, String)> {
    match line.split_whitespace().collect::<Vec<&str>>().first() {
        Some(first_word) => {
            if supported_commands().contains(&(*first_word).to_string()) {
                return Some((
                    (*first_word).to_string(),
                    line.replacen(&format!("{} ", first_word), "", 1),
                ));
            }
            None
        }
        None => None,
    }
}

fn arm_parsers() -> HashMap<String, fn(&mut ArmPreamble, &str)> {
    let mut parsers: HashMap<String, fn(&mut ArmPreamble, &str)> = HashMap::new();
    parsers.insert(String::from("FROM"), parse_arm_from);
    parsers.insert(String::from("CHECKSUM"), parse_arm_checksum);
    parsers
}

fn x86_parsers() -> HashMap<String, fn(&mut X86Preamble, &str)> {
    let mut parsers: HashMap<String, fn(&mut X86Preamble, &str)> = HashMap::new();
    parsers.insert(String::from("FROM"), parse_x86_from);
    parsers.insert(String::from("CHECKSUM"), parse_x86_checksum);
    parsers.insert(String::from("BOOTCMD"), parse_boot_cmd);
    parsers.insert(String::from("PRESEED"), parse_x86_preseed);
    parsers.insert(String::from("OBSERVE_BUILD"), parse_x86_set_headless);
    parsers.insert(String::from("VM_TYPE"), parse_x86_set_vm_type);
    parsers.insert(String::from("SSH_USER"), parse_x86_set_ssh_username);
    parsers.insert(String::from("SSH_PASSWORD"), parse_x86_set_ssh_password);
    parsers.insert(String::from("SHUTDOWN_CMD"), parse_x86_set_shutdown_cmd);
    parsers.insert(String::from("BOOT_TIME"), parse_x86_set_boot_time);
    parsers
}

fn common_parsers() -> HashMap<String, fn(&mut Imagefile, &str)> {
    let mut parsers: HashMap<String, fn(&mut Imagefile, &str)> = HashMap::new();
    parsers.insert(String::from("RUN"), parse_run);
    parsers.insert(String::from("FS"), parse_partition);
    parsers.insert(String::from("FILE"), parse_file);
    parsers.insert(String::from("CONFIG"), parse_config);
    parsers.insert(String::from("ON-DEVICE"), parse_on_device);
    parsers.insert(String::from("PREBUILT"), parse_prebuilt);
    parsers.insert(String::from("ENTRYPOINT"), parse_entrypoint);
    parsers.insert(String::from("PXE_KERNEL"), parse_pxe_kernel);
    parsers.insert(String::from("PXE_OPTIONS"), parse_pxe_options);
    parsers
}

fn supported_commands() -> Vec<String> {
    let mut vec = Vec::new();
    for str in &[
        "FROM",
        "RUN",
        "FS",
        "FILE",
        "CONFIG",
        "ON-DEVICE",
        "PREBUILT",
        "ENTRYPOINT",
        "ARCH",
        "BOOTCMD",
        "DISKSIZE",
        "CHECKSUM",
        "PRESEED",
        "OBSERVE_BUILD",
        "VM_TYPE",
        "SSH_USER",
        "SSH_PASSWORD",
        "SHUTDOWN_CMD",
        "BOOT_TIME",
        "PXE_KERNEL",
        "PXE_OPTIONS",
    ] {
        vec.push(String::from(*str))
    }
    vec
}

fn parse_arm_from(preamble: &mut ArmPreamble, line: &str) {
    match preamble.parse_base_image(line) {
        Ok(_) => {}
        Err(msg) => eprintln!("{}", msg),
    }
}
fn parse_x86_from(preamble: &mut X86Preamble, line: &str) {
    match preamble.parse_base_image(line) {
        Ok(_) => {}
        Err(msg) => eprintln!("{}", msg),
    }
}

fn parse_arm_checksum(preamble: &mut ArmPreamble, line: &str) {
    preamble.set_checksum(line.to_string());
}

fn parse_x86_checksum(preamble: &mut X86Preamble, line: &str) {
    preamble.set_checksum(line.to_string());
}

fn parse_run(image: &mut Imagefile, line: &str) {
    match Provisioner::parse(&SHELL, line) {
        Ok(provisioner) => image.provisioners.push(provisioner),
        Err(msg) => eprintln!("{}", msg),
    }
}

fn parse_file(image: &mut Imagefile, line: &str) {
    match Provisioner::parse(&FILE, line) {
        Ok(provisioner) => image.provisioners.push(provisioner),
        Err(msg) => eprintln!("{}", msg),
    }
}

fn parse_partition(image: &mut Imagefile, line: &str) {
    match Partition::parse(line) {
        Ok(partition) => {
            image
                .partitions
                .insert(partition.get_mountpoint(), partition);
        }
        Err(msg) => eprintln!("{}", msg),
    }
}

fn parse_config(image: &mut Imagefile, line: &str) {
    match BootConfig::parse(line) {
        Ok(config) => image.configuration.bootconfigs.push(config),
        Err(msg) => eprintln!("{}", msg),
    }
}

fn parse_on_device(image: &mut Imagefile, _line: &str) {
    image.configuration.on_device = true;
}

fn parse_prebuilt(image: &mut Imagefile, _line: &str) {
    image.configuration.prebuilt = true;
}

fn parse_entrypoint(image: &mut Imagefile, line: &str) {
    parse_run(image, "echo 'PATH=/sbin:/bin:/usr/sbin:/usr/bin' >> /root/cron");
    parse_run(image, "echo '@reboot sh /entrypoint.sh' >> /root/cron");
    parse_run(image, "/usr/bin/crontab /root/cron");
    parse_run(image, "echo '#!/usr/bin/env sh' > /entrypoint.sh");
    parse_run(image, "echo 'mkdir /results' >> /entrypoint.sh");
    parse_run(image, "echo 'date > /results/.started' >> /entrypoint.sh");
    parse_run(image, "echo 'curl --location --request POST '%LOG_SERVER%' --header 'Content-Type: text/plain' --data-raw \"started\"' >> /entrypoint.sh");
    parse_run(image, format!("echo '{}' >> /entrypoint.sh", line).as_str());
    parse_run(image, "echo 'echo $? > /results/.exited' >> /entrypoint.sh");
    parse_run(image, "echo 'curl --location --request POST '%LOG_SERVER%' --header 'Content-Type: text/plain' --data-raw \"exited\"' >> /entrypoint.sh");
    parse_run(image, "echo 'date >> /results/.exited' >> /entrypoint.sh");
    parse_run(image, "echo 'curl --location --request POST '%LOG_SERVER%' --header 'Content-Type: text/plain' --data-raw \"shutdown\"' >> /entrypoint.sh");
    parse_run(image, "echo 'shutdown now' >> /entrypoint.sh");
}

fn parse_boot_cmd(preamble: &mut X86Preamble, line: &str) {
    preamble.boot_command.push(line.to_string());
}

fn get_architecture(commands: &mut [(String, String)]) -> Option<Architecture> {
    let architectures = commands
        .iter()
        .filter_map(|(command, line)| {
            if command.eq("ARCH") {
                return match Architecture::parse(line.as_str()) {
                    Ok(arch) => Some(arch),
                    Err(msg) => {
                        eprintln!("{}", msg);
                        None
                    }
                };
            }
            None
        })
        .collect::<Vec<Architecture>>();
    if architectures.is_empty() || architectures.len() > 1 {
        None
    } else {
        Some(architectures.get(0).unwrap().to_owned())
    }
}

fn parse_x86_preseed(preamble: &mut X86Preamble, line: &str) {
    preamble.set_preseed_file(line.to_string());
}

fn parse_x86_set_headless(preamble: &mut X86Preamble, _line: &str) {
    preamble.set_headless(false);
}

fn parse_x86_set_vm_type(preamble: &mut X86Preamble, line: &str) {
    preamble.set_guest_os_type(line.to_string());
}

fn parse_x86_set_ssh_username(preamble: &mut X86Preamble, line: &str) {
    preamble.set_ssh_username(line.to_string());
}

fn parse_x86_set_ssh_password(preamble: &mut X86Preamble, line: &str) {
    preamble.set_ssh_password(line.to_string());
}

fn parse_x86_set_boot_time(preamble: &mut X86Preamble, line: &str) {
    preamble.set_boot_wait(line.to_string());
}

fn parse_x86_set_shutdown_cmd(preamble: &mut X86Preamble, line: &str) {
    preamble.set_shutdown_command(line.to_string());
}

fn parse_pxe_kernel(image: &mut Imagefile, line: &str) {
    image.configuration.pxe = true;
    image.configuration.pxe_kernel = line.to_string();
}
fn parse_pxe_options(image: &mut Imagefile, line: &str) {
    image.configuration.pxe = true;
    image.configuration.pxe_options = line.to_string();
}
