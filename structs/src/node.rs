use crate::architecture::Architecture;
use crate::logsource::LogSource;
use config::Value;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Node {
    pub id: String,
    pub name: String,
    pub tftp_prefix: String,
    pub mac_address: String,
    pub serial_number: String,
    pub ipv4_address: String,
    pub log_inputs: Vec<LogSource>,
    pub architecture: Architecture,
    pub pxe: bool,
}

impl Node {
    #[must_use]
    pub fn from_config(
        id: String,
        hash: &HashMap<String, Value>,
        log_inputs: Vec<LogSource>,
    ) -> Self {
        Node {
            id,
            name: hash.get("name").unwrap().to_owned().into_str().unwrap(),
            tftp_prefix: hash
                .get("tftp-prefix")
                .unwrap()
                .to_owned()
                .into_str()
                .unwrap(),
            mac_address: hash
                .get("mac-address")
                .unwrap()
                .to_owned()
                .into_str()
                .unwrap(),
            ipv4_address: hash
                .get("ipv4-address")
                .unwrap()
                .to_owned()
                .into_str()
                .unwrap(),
            serial_number: hash
                .get("serial-number")
                .unwrap()
                .to_owned()
                .into_str()
                .unwrap(),
            log_inputs,
            architecture: Architecture::parse(
                &hash
                    .get("architecture")
                    .unwrap()
                    .to_owned()
                    .into_str()
                    .unwrap(),
            )
            .unwrap(),
            pxe: if hash.contains_key("pxe") {
                hash.get("pxe")
                    .unwrap()
                    .to_owned()
                    .into_bool()
                    .unwrap_or(false)
            } else {
                false
            },
        }
    }
}

impl PartialEq for Node {
    fn eq(&self, other: &Self) -> bool {
        self.id.eq(&other.id)
            && self.name.eq(&other.name)
            && self.tftp_prefix.eq(&other.tftp_prefix)
            && self.mac_address.eq(&other.mac_address)
            && self.serial_number.eq(&other.serial_number)
    }
}
