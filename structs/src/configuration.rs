use crate::architecture::Architecture;
use crate::bootconfig::BootConfig;
use crate::mountpoint::Mountpoint;
use crate::partition::Partition;
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize)]
pub struct Configuration {
    pub name: String,
    pub architecture: Architecture,
    pub bootconfigs: Vec<BootConfig>,
    pub partitions: Vec<Partition>,
    pub mountorder: Vec<Mountpoint>,
    pub on_device: bool,
    pub prebuilt: bool,
    pub pxe: bool,
    pub pxe_kernel: String,
    pub pxe_options: String,
}

impl Configuration {
    pub fn mountorder_to_vec(&mut self, mountorder: HashMap<String, Mountpoint>) -> bool {
        if mountorder.is_empty() {
            return false;
        }
        let mut values = Vec::new();
        for (_, mountpoint) in mountorder {
            values.push(mountpoint);
        }
        self.mountorder = values;
        true
    }

    pub fn partitions_to_vec(&mut self, partitions: &HashMap<String, Partition>) -> bool {
        if partitions.is_empty() {
            return false;
        }
        let mut values = Vec::new();
        let sorted = partitions.iter().sorted_by(|(_, p1), (_, p2)| p1.cmp(p2));
        for (_, partition) in sorted {
            values.push(partition.clone());
        }
        values.sort_by_key(Partition::get_start);
        self.partitions = values;
        true
    }

    pub fn merge(&mut self, other: Configuration) {
        for bootconfig in other.bootconfigs {
            self.bootconfigs.push(bootconfig);
        }
        for partition in other.partitions {
            if !self.partitions.contains(&partition) {
                self.partitions.push(partition);
            }
        }
        for mountorder in other.mountorder {
            if !self.mountorder.contains(&mountorder) {
                self.mountorder.push(mountorder);
            }
        }
        self.on_device = other.on_device;
        self.pxe = other.pxe;
        if self.pxe {
            if self.pxe_kernel.is_empty() {
                self.pxe_kernel = other.pxe_kernel;
            }
            if self.pxe_options.is_empty() {
                self.pxe_options = other.pxe_options;
            }
        }
    }
}

impl Default for Configuration {
    fn default() -> Self {
        Self {
            name: String::new(),
            architecture: Architecture::ARM64,
            bootconfigs: Vec::new(),
            partitions: Vec::new(),
            mountorder: Vec::new(),
            on_device: false,
            prebuilt: false,
            pxe: false,
            pxe_kernel: String::new(),
            pxe_options: String::new(),
        }
    }
}
