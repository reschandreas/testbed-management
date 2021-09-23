use crate::service::Service;
use crate::task::Task;
use crate::task::Type::StopIfTrue;
use chrono::{NaiveDateTime, Utc};
use rusqlite::Row;
use serde::{Deserialize, Serialize};
use yaml_rust::Yaml;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Deployment {
    pub id: Option<i64>,
    pub name: String,
    pub services: Vec<Service>,
    pub owner: String,
    pub start: NaiveDateTime,
    pub end: Option<NaiveDateTime>,
    pub tasks: Vec<Task>,
}

impl Deployment {
    #[must_use]
    pub fn new(name: &str) -> Self {
        Deployment {
            id: None,
            name: String::from(name),
            services: Vec::new(),
            owner: String::from("aresch"),
            start: Utc::now().naive_local(),
            end: None,
            tasks: Vec::new(),
        }
    }

    #[must_use]
    pub fn from_yaml(name: &str, yaml: &Yaml) -> Self {
        let mut services: Vec<Service> = Vec::new();
        for (n, data) in yaml["services"].as_hash().unwrap().iter() {
            services.push(Service::from_yaml(
                n.as_str().unwrap(),
                data.as_hash().unwrap(),
            ));
        }
        let mut tasks = Vec::new();
        if let Some (tasks_yaml) = yaml["stop"].as_hash() {
            for (task, data) in  tasks_yaml.iter() {
                if task.as_str().unwrap().eq("log") {
                    for task in data.to_owned().into_vec().unwrap() {
                        let msg_yaml = task["message"].to_owned();
                        let message = msg_yaml.as_str().unwrap();
                        let occurrence = task["occurrence"].to_owned().as_i64().unwrap();
                        let task = Task::new(
                            None,
                            None,
                            StopIfTrue,
                            serde_json::to_string(&(message, occurrence)).unwrap(),
                            true,
                        );
                        tasks.push(task);
                    }
                }
            }
        }
        Deployment {
            id: None,
            name: String::from(name),
            services,
            owner: String::from("aresch"),
            start: Utc::now().naive_local(),
            end: None,
            tasks,
        }
    }

    #[must_use]
    pub fn from_row(row: &Row) -> Self {
        Deployment {
            id: row.get(0).unwrap(),
            name: row.get(1).unwrap(),
            services: Vec::new(),
            owner: row.get(2).unwrap(),
            start: row.get(3).unwrap(),
            end: row.get(4).unwrap(),
            tasks: Vec::new(),
        }
    }

    #[must_use]
    pub fn get_services(&self) -> Vec<Service> {
        self.services.clone()
    }
}
