use crate::utils;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use string_builder::Builder;

#[derive(Debug, Serialize, Deserialize, Eq, Clone)]
pub struct Partition {
    filesystem: String,
    mountpoint: String,
    name: String,
    size: String,
    start_sector: String,
    partition_type: String,
}

impl Partition {
    /// # Errors
    ///
    /// Will return `Err` if `line` could not be parsed
    pub fn parse(line: &str) -> Result<Partition, &'static str> {
        let parts = line.split_whitespace().collect::<Vec<&str>>();
        if parts.len() == 6 {
            let filesystem = String::from(parts[0]);
            let mountpoint = String::from(parts[1]);
            let name = String::from(parts[2]);
            let size = String::from(parts[3]);
            let start_sector = String::from(parts[4]);
            let partition_type = String::from(parts[5]);
            return Ok(Partition {
                filesystem,
                mountpoint,
                name,
                size,
                start_sector,
                partition_type,
            });
        }
        Err("Could not parse Partition")
    }

    #[must_use]
    pub fn get_values(&self) -> Vec<(&'static str, String)> {
        let mut fields = Vec::new();
        fields.push(("filesystem", utils::quote(&self.filesystem)));
        fields.push(("mountpoint", utils::quote(&self.mountpoint)));
        fields.push(("name", utils::quote(&self.name)));
        fields.push(("size", utils::quote(&self.size)));
        fields.push(("start_sector", utils::quote(&self.start_sector)));
        fields.push(("type", utils::quote(&self.partition_type)));
        fields
    }

    #[must_use]
    pub fn to_pkr_hcl(&self) -> String {
        let mut builder = Builder::default();
        utils::ident_and_append(&mut builder, "image_partitions {\n", 2);
        for (key, value) in &self.get_values() {
            utils::add_indented_aligned_key_value(&mut builder, 4, 20, key, &value);
        }
        utils::ident_and_append(&mut builder, "}\n", 2);
        builder.string().unwrap()
    }

    #[must_use]
    pub fn get_mountpoint(&self) -> String {
        self.mountpoint.clone()
    }

    #[must_use]
    pub fn get_name(&self) -> String {
        self.name.clone()
    }

    #[must_use]
    pub fn get_start(&self) -> usize {
        self.start_sector.parse::<usize>().unwrap()
    }
}

impl PartialOrd for Partition {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Partition {
    fn cmp(&self, other: &Self) -> Ordering {
        let start = self.start_sector.parse::<i64>().unwrap();
        let start_other = other.start_sector.parse::<i64>().unwrap();
        start.cmp(&start_other)
    }
}

impl PartialEq for Partition {
    fn eq(&self, other: &Self) -> bool {
        self.start_sector == other.start_sector
    }
}
