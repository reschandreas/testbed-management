use crate::installer::BASE_DIR;
use crate::node_manager::verify_node_usability;
use config::{Config, Value};
use std::collections::hash_map::RandomState;
use std::collections::HashMap;
use structs::logsource::LogSourceTypes::{HOST, SERIAL};
use structs::logsource::{LogSource, LogSourceTypes};
use structs::node::Node;
use structs::power_action::PowerAction;
use structs::power_action::Type::{OFF, ON, REBOOT};
use structs::power_action_set::PowerActionSet;

fn read_config() -> Config {
    let mut config = Config::default();
    config
        .merge(config::File::with_name(
            format!("{}/{}", BASE_DIR, "config.yml").as_str(),
        ))
        .unwrap();
    config
}

pub fn get(property: &str) -> Option<String> {
    let config = read_config();
    match config.get::<String>(property) {
        Ok(str) => Some(str),
        Err(_) => None,
    }
}

#[allow(dead_code)]
pub fn get_or_default(property: &str, default: &str) -> String {
    let config = read_config().try_into::<HashMap<String, String>>().unwrap();
    match config.get(property) {
        Some(str) => String::from(str),
        None => String::from(default),
    }
}

pub fn get_log_sources_of(node: &Node) -> Vec<LogSource> {
    let mut vec = get_log_sources(node.id.as_str());
    vec.push(LogSource::new(node.ipv4_address.clone(), HOST));
    vec
}

fn get_log_sources(id: &str) -> Vec<LogSource> {
    let config = read_config();
    let mut vec = Vec::new();
    let nodes = config.get_table("nodes").unwrap();
    if let Some(node) = nodes.get(id) {
        if let Ok(node_table) = node.to_owned().into_table() {
            if let Some(log_inputs) = node_table.get("log-inputs") {
                if let Ok(log_inputs_table) = log_inputs.to_owned().into_table() {
                    for tuple in [("hosts", HOST), ("serial", SERIAL)].to_vec() {
                        if let Some(entries) = log_inputs_table.get(tuple.0) {
                            if let Ok(array) = entries.to_owned().into_array() {
                                vec.append(&mut get_log_source_from_array(array, &tuple.1));
                            }
                        }
                    }
                }
            }
        }
    }
    vec
}

pub fn get_nodes() -> Result<Vec<Node>, &'static str> {
    match get_all_nodes() {
        Ok(all_nodes) => {
            let mut vec = Vec::new();
            for node in all_nodes {
                if verify_node_usability(&node) {
                    vec.push(node);
                }
            }
            Ok(vec)
        }
        Err(s) => Err(s),
    }
}

pub fn get_all_nodes() -> Result<Vec<Node>, &'static str> {
    let config = read_config();
    let mut vec: Vec<Node> = Vec::new();
    let nodes = config.get_table("nodes").unwrap();
    for (id, values) in nodes {
        vec.push(Node::from_config(
            id.clone(),
            &values.into_table().unwrap(),
            get_log_sources(id.as_str()),
        ));
    }
    if vec.is_empty() {
        Err("no nodes")
    } else {
        Ok(vec)
    }
}

pub fn get_node_by_id(id: &str, all: bool) -> Option<Node> {
    let nodes = if all { get_all_nodes() } else { get_nodes() };
    if let Ok(nodes) = nodes {
        let filtered = nodes
            .into_iter()
            .filter(|n| n.id.eq(&id))
            .collect::<Vec<Node>>();
        return match filtered.first() {
            None => None,
            Some(node) => Some(node.to_owned()),
        };
    }
    None
}

fn get_log_source_from_array(source: Vec<Value>, logtype: &LogSourceTypes) -> Vec<LogSource> {
    let mut vec = Vec::new();
    for path in source {
        match logtype {
            LogSourceTypes::HOST => vec.push(LogSource::host(path.into_str().unwrap())),
            LogSourceTypes::SERIAL => vec.push(LogSource::serial(path.into_str().unwrap())),
        }
    }
    vec
}

fn get_power_table_of(node: &Node) -> Option<HashMap<String, Value, RandomState>> {
    let config = read_config();
    let nodes = config.get_table("nodes").unwrap();
    if let Some(node) = nodes.get(&node.id) {
        if let Ok(node_table) = node.to_owned().into_table() {
            if let Some(power_actions) = node_table.get("power") {
                if let Ok(power_actions_table) = power_actions.to_owned().into_table() {
                    return Some(power_actions_table);
                }
            }
        }
    }
    Option::None
}

pub fn get_power_commands_of(node: &Node) -> PowerActionSet {
    let mut on: Result<PowerAction, String> = Err("no on command".to_string());
    let mut off: Result<PowerAction, String> = Err("no off command".to_string());
    let mut reboot: Result<PowerAction, String> = Err("no reboot command".to_string());
    if let Some(power_actions_table) = get_power_table_of(node) {
        for (key, t) in [("on", ON), ("off", OFF), ("reboot", REBOOT)].to_vec() {
            if let Some(entry) = power_actions_table.get(key) {
                let command = entry.to_owned().into_str().unwrap();
                match t {
                    ON => on = PowerAction::parse(ON, &command),
                    OFF => off = PowerAction::parse(OFF, &command),
                    REBOOT => reboot = PowerAction::parse(REBOOT, &command),
                }
            }
        }
    }
    PowerActionSet::new(on, off, reboot)
}

pub fn get_default_os_for(node: &Node) -> Option<String> {
    let config = read_config();
    let nodes = config.get_table("nodes").unwrap();
    if let Some(node) = nodes.get(&node.id) {
        if let Ok(node_table) = node.to_owned().into_table() {
            if let Some(default_os) = node_table.get("default-os") {
                if let Ok(os) = default_os.to_owned().into_str() {
                    return Some(os);
                }
            }
        }
    }
    Option::None
}

pub fn get_default_user_for(node: &Node) -> Option<String> {
    let config = read_config();
    let nodes = config.get_table("nodes").unwrap();
    if let Some(node) = nodes.get(&node.id) {
        if let Ok(node_table) = node.to_owned().into_table() {
            if let Some(default_user) = node_table.get("default-user") {
                if let Ok(os) = default_user.to_owned().into_str() {
                    return Some(os);
                }
            }
        }
    }
    Option::None
}

pub fn get_storage_device_of(node: &Node) -> Option<String> {
    let config = read_config();
    let nodes = config.get_table("nodes").unwrap();
    if let Some(node) = nodes.get(&node.id) {
        if let Ok(node_table) = node.to_owned().into_table() {
            if let Some(storage) = node_table.get("storage-device") {
                if let Ok(os) = storage.to_owned().into_str() {
                    return Some(os);
                }
            }
        }
    }
    Option::None
}
