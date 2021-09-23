use crate::architecture::Architecture;
use crate::utils::get_random_name;
use chrono::{NaiveDateTime, Utc};
use rusqlite::Row;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::convert::TryInto;
use yaml_rust::yaml::Hash;
use yaml_rust::Yaml;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Service {
    pub id: Option<i64>,
    pub name: String,
    pub image: String,
    pub hostname: String,
    pub replicas: i64,
    pub deployment: Option<i64>,
    pub ipv4_address: Option<String>,
    pub preferred_node: Option<String>,
    pub start: NaiveDateTime,
    pub end: Option<NaiveDateTime>,
    pub node: Option<String>,
    pub architecture: Option<Architecture>,
}

impl Service {
    #[must_use]
    pub fn new(name: &str, image: &str, hostname: &str) -> Self {
        Service {
            id: None,
            name: String::from(name),
            image: String::from(image),
            hostname: String::from(hostname),
            replicas: 1,
            deployment: None,
            ipv4_address: None,
            preferred_node: None,
            start: Utc::now().naive_local(),
            end: None,
            node: None,
            architecture: None,
        }
    }

    #[must_use]
    pub fn from_yaml(name: &str, hash: &Hash) -> Self {
        let image = String::from(
            hash.get(&Yaml::from_str("image"))
                .unwrap()
                .as_str()
                .unwrap(),
        );
        let hostname = match hash.get(&Yaml::from_str("hostname")) {
            None => get_random_name(),
            Some(value) => String::from(value.as_str().unwrap_or_default()),
        };
        let replicas = match hash.get(&Yaml::from_str("replicas")) {
            Some(r) => r.as_i64().unwrap(),
            None => 1,
        };
        let node = match hash.get(&Yaml::from_str("node")) {
            Some(n) => Some(n.as_str().unwrap().to_string()),
            None => None,
        };
        let ipv4_address = match hash.get(&Yaml::from_str("ipv4-address")) {
            Some(ip) => Some(ip.as_str().unwrap().to_string()),
            None => None,
        };
        Service {
            id: None,
            name: String::from(name),
            image,
            hostname,
            replicas,
            deployment: None,
            ipv4_address,
            preferred_node: node,
            start: Utc::now().naive_local(),
            end: None,
            node: None,
            architecture: None,
        }
    }

    #[must_use]
    pub fn from_row(row: &Row) -> Self {
        let arch: String = row.get(9).unwrap();
        Service {
            id: row.get(0).unwrap(),
            name: row.get(1).unwrap(),
            image: row.get(2).unwrap(),
            deployment: row.get(3).unwrap(),
            node: row.get(4).unwrap(),
            start: row.get(5).unwrap(),
            end: row.get(6).unwrap(),
            ipv4_address: row.get(7).unwrap(),
            hostname: row.get(8).unwrap(),
            replicas: 1,
            preferred_node: None,
            architecture: Some(Architecture::parse(&arch).unwrap_or(Architecture::ARM64)),
        }
    }

    #[must_use]
    pub fn group_services(services: Vec<Service>) -> Vec<Service> {
        let mut groups: HashMap<String, Vec<Service>> = HashMap::new();
        for service in services {
            let key = format!("{}-{}", service.image, service.deployment.unwrap());
            let mut value = match groups.get(key.as_str()) {
                Some(v) => v.clone(),
                None => Vec::new(),
            };
            value.push(service);
            groups.insert(key, value);
        }
        let mut vec = Vec::new();
        for (_, value) in groups {
            let mut first = value.first().unwrap().clone();
            first.replicas = value.len().try_into().unwrap();
            vec.push(first);
        }
        vec
    }
}
