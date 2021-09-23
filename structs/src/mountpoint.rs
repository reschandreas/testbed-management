use crate::utils::remove_colors;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;

#[derive(Debug, Serialize, Deserialize, Eq, Clone)]
pub struct Mountpoint {
    mount_position: usize,
    pub partition_number: usize,
    path: String,
}

impl Mountpoint {
    #[must_use]
    pub fn new(mount_position: usize, partition_number: usize, path: String) -> Self {
        Mountpoint {
            mount_position,
            partition_number,
            path,
        }
    }

    #[must_use]
    pub fn sort(a: &Mountpoint, b: &Mountpoint) -> Ordering {
        a.mount_position.cmp(&b.mount_position)
    }

    #[must_use]
    pub fn get_path(&self) -> String {
        if self.path.is_empty() {
            return String::from('/');
        } else if !self.path.starts_with('/') {
            return format!("/{}", self.path);
        }
        self.path.clone()
    }
}

#[must_use]
pub fn get_mount_order(output: &str) -> Vec<Mountpoint> {
    let mut list = output
        .lines()
        .filter_map(|s| {
            if s.contains("mounting") && s.contains(" to ") {
                Some(
                    remove_colors(s)
                        .split_whitespace()
                        .filter_map(|s| {
                            if s.contains('/') {
                                Some(s.to_string())
                            } else {
                                None
                            }
                        })
                        .collect::<Vec<String>>(),
                )
            } else {
                None
            }
        })
        .filter_map(|vec| {
            if vec.len() == 2 {
                let mut path_vec = vec[1].split('/').collect::<Vec<&str>>();
                path_vec.drain(1..3);
                let mut path = path_vec.join("/");
                if path.is_empty() {
                    path = String::from("/");
                }
                Some(Mountpoint::new(
                    path_vec.len(),
                    vec[0].split('p').collect::<Vec<&str>>()[2].parse().unwrap(),
                    path,
                ))
            } else {
                None
            }
        })
        .collect::<Vec<Mountpoint>>();
    list.sort_by(|a, b| Mountpoint::sort(a, b));
    list
}

impl PartialOrd for Mountpoint {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Mountpoint {
    fn cmp(&self, other: &Self) -> Ordering {
        self.mount_position.cmp(&other.mount_position)
    }
}

impl PartialEq for Mountpoint {
    fn eq(&self, other: &Self) -> bool {
        self.mount_position.eq(&other.mount_position)
            && self.path.eq(&other.path)
            && self.partition_number.eq(&other.partition_number)
    }
}
