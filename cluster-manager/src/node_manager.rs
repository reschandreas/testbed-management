extern crate clap;
use crate::config::{get_all_nodes, get_log_sources_of, get_node_by_id};
use crate::database::get_running_services;
use crate::deployer::clean_node;
use crate::installer::{
    DNSMASQ, DNSMASQ_NODES_CONFIG_FILE, NFS_BASE_DIR, NFS_SERVICE, PING, SCREEN, SERVICE,
    TFTP_BASE_DIR, UMOUNT,
};
use crate::logs_manager::gather_logs;
use colored::Colorize;
use prettytable::format;
use prettytable::{Cell, Row, Table};
use std::path::Path;
use std::process::Command;
use std::sync::mpsc;
use std::sync::mpsc::Sender;
use std::thread;
use std::thread::JoinHandle;
use std::{fs, io};
use structs::logsource::LogSourceTypes;
use structs::node::Node;
use structs::node_row::NodeRow;
use structs::service::Service;
use structs::utils::{
    append_to_file, filter_lines_by_substring, get_lines_from_file, print_information,
    print_message, remove_line_from_file, remove_line_with_substring_from_file,
};

const NFS_CONFIGFILE: &str = "/etc/exports";

pub fn add_node(identifier: &str) -> bool {
    if let Some(node) = get_node_by_id(identifier, true) {
        add_nfs(&node);
        add_dnsmasq(&node.ipv4_address, &node);
        add_tftp(&node);
        restart_services();
        true
    } else {
        eprintln!("Add node to configuration first");
        false
    }
}

pub fn remove_node(node: &Node) -> bool {
    stop_node(&node, false, true);
    remove_nfs(&node);
    remove_tftp(&node);
    print_information("This node can now safely be removed from the configuration");
    true
}

pub fn restart_services() {
    for service in &[NFS_SERVICE, DNSMASQ] {
        let status = Command::new(SERVICE)
            .arg(service)
            .arg("restart")
            .output()
            .expect(&*format!("failed to restart {}", service))
            .status
            .success();
        print_message(format!("restarting {}", service).as_str(), status);
    }
}

fn add_nfs(node: &Node) {
    print_message(
        "create nfs directory",
        create_nfs_share(&node.tftp_prefix.as_str()).is_ok(),
    );
    print_message("add nfs share", add_nfsshare(&node.tftp_prefix).is_ok());
}

fn add_tftp(node: &Node) {
    print_message(
        "add tftp directory",
        add_tftp_directory(&node.tftp_prefix).is_ok(),
    );
}

fn add_tftp_directory(prefix: &str) -> io::Result<()> {
    fs::create_dir(format!("{}/{}", TFTP_BASE_DIR, prefix))
}

fn create_nfs_share(prefix: &str) -> io::Result<()> {
    fs::create_dir(format!("{}/{}", NFS_BASE_DIR, prefix))
}

fn add_nfsshare(prefix: &str) -> io::Result<()> {
    let line = format!(
        "{}/{} *(rw,sync,no_subtree_check,no_root_squash)",
        NFS_BASE_DIR, prefix
    );
    append_to_file(NFS_CONFIGFILE, line)
}

fn add_dnsmasq_ip(ipv4_address: &str, node: &Node) -> io::Result<()> {
    let line = format!(
        "dhcp-host={}{},{},{}",
        if node.pxe { "set:pxe," } else { "" },
        node.mac_address,
        ipv4_address,
        node.name
    );
    append_to_file(DNSMASQ_NODES_CONFIG_FILE, line)
}

fn add_dnsmasq(ipv4_address: &str, node: &Node) {
    print_message(
        "add dnsmasq entry",
        add_dnsmasq_ip(ipv4_address, &node).is_ok(),
    );
}

fn remove_nfs(node: &Node) {
    print_message(
        "remove nfs share",
        remove_nfsshare_entry(&node.tftp_prefix).is_ok(),
    );
    print_message(
        "remove nfs directory",
        remove_nfs_share_directory(&node.tftp_prefix).is_ok(),
    );
}

fn remove_dnsmasq(node: &Node) {
    print_message("remove dnsmasq entry", remove_dnsmasq_entry(&node).is_ok());
}

fn remove_tftp(node: &Node) {
    print_message(
        "unmount tftpboot directory",
        umount_tftp_directory(&node.tftp_prefix),
    );
    print_message(
        "remove tftpboot directory",
        remove_tftp_directory(&node.tftp_prefix).is_ok(),
    );
}

fn remove_nfs_share_directory(prefix: &str) -> io::Result<()> {
    fs::remove_dir_all(format!("{}/{}", NFS_BASE_DIR, prefix))
}

fn remove_nfsshare_entry(prefix: &str) -> io::Result<()> {
    let line = format!(
        "{}/{} *(rw,sync,no_subtree_check,no_root_squash)",
        NFS_BASE_DIR, prefix
    );
    remove_line_from_file(NFS_CONFIGFILE, &line)
}

pub fn umount_tftp_directory(prefix: &str) -> bool {
    Command::new(UMOUNT)
        .arg(format!("{}/{}", TFTP_BASE_DIR, prefix))
        .output()
        .expect("failed to unmount tftpboot")
        .status
        .success()
}

fn remove_tftp_directory(prefix: &str) -> io::Result<()> {
    fs::remove_dir_all(format!("{}/{}", TFTP_BASE_DIR, prefix))
}

fn remove_dnsmasq_entry(node: &Node) -> io::Result<()> {
    let line = format!(
        "dhcp-host={}{}",
        if node.pxe { "set:pxe," } else { "" },
        node.mac_address
    );
    remove_line_with_substring_from_file(DNSMASQ_NODES_CONFIG_FILE, &line)
}

pub fn change_hostname(node: &mut Node, hostname: &str) -> bool {
    if let Ok(content) = get_lines_from_file(DNSMASQ_NODES_CONFIG_FILE) {
        let lines = filter_lines_by_substring(&content, &node.mac_address);
        if lines.len() == 1 {
            let mut parts = lines[0].split(',').collect::<Vec<&str>>();
            parts.drain(..(parts.len() - 2));
            let ipv4_address = parts[0];
            let old_hostname = parts[1];

            print_information(&format!("ipv4address is {}", ipv4_address.green()));
            print_information(&format!("old hostname was {}", old_hostname));
            print_information(&format!("new hostname is {}", hostname.green()));
            remove_dnsmasq(&node);
            node.name = hostname.to_string();
            add_dnsmasq(ipv4_address, &node);
            return true;
        }
    }
    false
}

pub fn get_ipv4_address(node: &Node) -> Option<String> {
    if let Ok(content) = get_lines_from_file(DNSMASQ_NODES_CONFIG_FILE) {
        let lines = filter_lines_by_substring(&content, &node.mac_address);
        if lines.len() == 1 {
            let mut parts = lines[0].split(',').collect::<Vec<&str>>();
            parts.drain(..(parts.len() - 2));
            return Some(String::from(parts[0]));
        }
    }
    None
}

pub fn change_ipv4address(node: &mut Node, ipv4_address: &str) -> bool {
    remove_dnsmasq(&node);
    add_dnsmasq(ipv4_address, &node);
    true
}

pub fn list_nodes(show_all: bool) {
    let mut table = Table::new();
    table.set_format(*format::consts::FORMAT_NO_BORDER_LINE_SEPARATOR);
    table.set_titles(Row::new(
        [
            Cell::new("id"),
            Cell::new("name"),
            Cell::new("MAC-Address"),
            Cell::new("TFTP-Prefix"),
            Cell::new("serial-number"),
            Cell::new("status"),
            Cell::new("hostname"),
            Cell::new("IPv4-address"),
            Cell::new("usable"),
        ]
        .to_vec(),
    ));
    let rows = get_nodes_rows(show_all);
    for row in &rows {
        table.add_row(Row::new(row.get_cells()));
    }
    println!(
        "The cluster consists of {} nodes, {} are up.",
        get_all_nodes().unwrap().len(),
        rows.into_iter()
            .filter(|s| s.status.is_some() && s.status.unwrap())
            .count()
    );
    table.printstd();
}

fn get_node_line_handle(
    show_all: bool,
    line: Vec<(String, String, String)>,
    services: Vec<Service>,
    tx: Sender<Option<NodeRow>>,
    node: Node,
) -> JoinHandle<()> {
    thread::spawn(move || {
        let (hostname, ipv4_address) = match line
            .iter()
            .filter(|(_, _, mac)| mac.eq(&node.mac_address))
            .collect::<Vec<&(String, String, String)>>()
            .first()
        {
            Some((name, ip, _)) => (Some(name.to_string()), Some(ip.to_string())),
            None => (None, None),
        };
        let row = NodeRow::new(
            node.clone(),
            match services
                .iter()
                .filter(|s| s.node.eq(&Some(node.id.clone())))
                .collect::<Vec<&Service>>()
                .first()
            {
                Some(s) => s
                    .ipv4_address
                    .as_ref()
                    .map(|ipv4_address| is_up(ipv4_address.as_str())),
                None => None,
            },
            hostname,
            ipv4_address,
            verify_node_usability(&node),
        );
        if show_all || row.status.unwrap_or(false) {
            tx.send(Some(row)).unwrap();
        } else {
            tx.send(None).unwrap();
        }
    })
}

pub fn is_up(ipv4_address: &str) -> bool {
    Command::new(PING)
        .arg("-c 1")
        .arg("-v4")
        .arg(ipv4_address)
        .output()
        .expect("failed to execute ping")
        .status
        .success()
}

pub fn stop_node(node: &Node, prune: bool, hard_delete: bool) -> bool {
    print_information("stopping logging from serial inputs");
    close_screens_for_serial_logging(&node);
    gather_logs(node);
    if prune {
        print_message("clean node", clean_node(node));
    }
    print_message(
        "unmount tftpboot directory",
        umount_tftp_directory(node.tftp_prefix.as_str()),
    );
    print_message(
        "remove filesystem",
        remove_nfsroot(node.tftp_prefix.as_str(), hard_delete).is_ok(),
    );
    print_message(
        "remove pxefile",
        fs::remove_file(&format!(
            "{}/pxelinux.cfg/01-{}",
            TFTP_BASE_DIR,
            node.mac_address.replace(":", "-")
        ))
        .is_ok(),
    );
    restart_services();
    true
}

pub fn remove_nfsroot(node_mac: &str, hard_delete: bool) -> io::Result<()> {
    let result = fs::remove_dir_all(format!("{}/{}/", NFS_BASE_DIR, node_mac));
    if result.is_ok() && !hard_delete {
        return fs::create_dir_all(format!("{}/{}/", NFS_BASE_DIR, node_mac));
    }
    result
}

fn close_screens_for_serial_logging(node: &Node) {
    for (index, _serial_device) in get_log_sources_of(&node)
        .iter()
        .filter(|l| l.source.eq(&LogSourceTypes::SERIAL))
        .enumerate()
    {
        //screen -X -S rpi2 quit
        let name = format!("{}-{}", &node.id, index);
        let status = Command::new(SCREEN)
            .arg("-X")
            .arg("-S")
            .arg(&name)
            .arg("quit")
            .spawn()
            .unwrap()
            .wait()
            .unwrap()
            .success();
        print_message(&format!("stopping screen {}", &name), status);
    }
}

pub fn verify_node_usability(node: &Node) -> bool {
    let nfs_path = Path::new(&format!("{}/{}", NFS_BASE_DIR, node.tftp_prefix)).exists();
    let tftp_path = Path::new(&format!("{}/{}", TFTP_BASE_DIR, node.tftp_prefix)).exists();
    let dnsmasq_entry = match get_lines_from_file(DNSMASQ_NODES_CONFIG_FILE) {
        Ok(lines) => !filter_lines_by_substring(&lines, &node.mac_address).is_empty(),
        Err(_) => false,
    };
    nfs_path && tftp_path && dnsmasq_entry
}

pub fn get_nodes_rows(show_all: bool) -> Vec<NodeRow> {
    let services = match get_running_services() {
        Ok(s) => s,
        Err(_) => Vec::new(),
    };
    let lines = match get_lines_from_file(DNSMASQ_NODES_CONFIG_FILE) {
        Ok(content) => content
            .into_iter()
            .filter_map(|line| {
                if line.starts_with("dhcp-host=") {
                    let mut vec = line
                        .replace("dhcp-host=", "")
                        .split(',')
                        .map(std::string::ToString::to_string)
                        .collect::<Vec<String>>();
                    if vec.len() >= 3 {
                        vec.drain(..(vec.len() - 3));
                        Some((vec[2].to_string(), vec[1].to_string(), vec[0].to_string()))
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .collect::<Vec<(String, String, String)>>(),
        Err(_) => Vec::new(),
    };
    let mut rows = Vec::new();
    if let Ok(nodes) = get_all_nodes() {
        let mut handles: Vec<JoinHandle<()>> = Vec::new();
        let (tx, rx) = mpsc::channel();
        for n in &nodes {
            handles.push(get_node_line_handle(
                show_all,
                lines.clone(),
                services.clone(),
                tx.clone(),
                n.clone(),
            ));
        }
        for handle in handles {
            handle.join().unwrap();
            if let Some(row) = rx.recv().unwrap() {
                rows.push(row);
            }
        }
    }
    rows
}
