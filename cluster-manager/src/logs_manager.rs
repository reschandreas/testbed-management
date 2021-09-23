use crate::config::{get, get_log_sources_of, get_node_by_id};
use crate::installer::{COPY, MOVE, NFS_BASE_DIR, RESULTS_DIR, ZIP};
use linemux::MuxedLines;
use serde_json::Value;
use std::collections::HashMap;
use std::fs;
use std::ops::Add;
use std::path::Path;
use std::process::Command;
use structs::deployment::Deployment;
use structs::logsource::LogSourceTypes::HOST;
use structs::logsource::{LogSource, LogSourceTypes};
use structs::node::Node;
use structs::service::Service;
use structs::utils::get_lines_from_file;
use tokio::sync::mpsc::Sender;

fn get_log_base_directory() -> Option<String> {
    if let Some(config_value) = get("logstash-base-directory") {
        let logstash_directory = if config_value.ends_with('/') {
            config_value
        } else {
            config_value.add("/")
        };
        return Some(logstash_directory);
    }
    None
}

pub fn collect_deployment_logs(id: i64, services: Vec<Service>) -> bool {
    let deployment_dir = format!("{}/{}/", RESULTS_DIR, id);
    fs::create_dir_all(&deployment_dir).unwrap();
    for service in services {
        let logs = format!("{}/{}", RESULTS_DIR, service.node.unwrap());
        let logs_path = Path::new(&logs);
        if logs_path.exists() {
            Command::new(MOVE)
                .arg(logs)
                .arg(&deployment_dir)
                .spawn()
                .expect("failed to copy service logs to deployments logs")
                .wait()
                .unwrap()
                .success();
        }
    }
    let mut child = Command::new(ZIP)
        .current_dir(format!("{}/", RESULTS_DIR))
        .arg("-r")
        .arg(format!("{}.zip", id))
        .arg(format!("./{}", id))
        .spawn()
        .expect("failed to zip logs");
    child.wait().unwrap().success()
}

pub fn gather_logs(node: &Node) -> bool {
    let log_sources = get_log_sources_of(&node);
    let nfs_dir = format!("{}/{}/results/", NFS_BASE_DIR, node.id);
    fs::create_dir_all(format!("{}/{}/logs", RESULTS_DIR, node.id)).unwrap();
    let nfs_path = Path::new(&nfs_dir);
    if nfs_path.exists() {
        Command::new(COPY)
            .arg("-a")
            .arg(nfs_dir)
            .arg(format!("{}/{}/logs/results", RESULTS_DIR, node.id))
            .spawn()
            .expect("failed to copy content of /results to results directory")
            .wait()
            .unwrap()
            .success();
    };
    for log_source in log_sources {
        match log_source.source {
            LogSourceTypes::HOST => {
                Command::new(MOVE)
                    .arg(format!(
                        "{}logs/{}/logs",
                        get_log_base_directory().unwrap(),
                        log_source.path
                    ))
                    .arg(format!(
                        "{}/{}/logs/{}.log",
                        RESULTS_DIR, node.id, log_source.path
                    ))
                    .spawn()
                    .expect("failed to copy logs to results directory")
                    .wait()
                    .unwrap()
                    .success();
            }
            LogSourceTypes::SERIAL => {}
        }
    }
    true
}

fn get_log_sources(log_sources: Vec<LogSource>) -> (Vec<String>, HashMap<String, LogSourceTypes>) {
    let mut files = Vec::new();
    let mut file_to_types: HashMap<String, LogSourceTypes> = HashMap::new();
    for log_source in log_sources {
        match log_source.source {
            LogSourceTypes::HOST => {
                let file = format!(
                    "{}logs/{}/logs",
                    get_log_base_directory().unwrap(),
                    log_source.path
                );
                file_to_types.insert(file.clone(), LogSourceTypes::HOST);
                files.push(file);
            }
            LogSourceTypes::SERIAL => {}
        }
    }
    (files, file_to_types)
}

pub fn get_logs_of_deployment(
    deployment: &Deployment,
    timestamp: bool,
) -> Vec<(String, String, String)> {
    let mut vec = Vec::new();
    for service in &deployment.services {
        if let Some(node) = get_node_by_id(service.node.as_ref().unwrap(), false) {
            let log_sources = get_log_sources_of(&node);
            let (files, file_to_types) = get_log_sources(log_sources);
            for file in files {
                if let Ok(file_content) = get_lines_from_file(&file) {
                    for line in file_content {
                        match file_to_types.get(&file).unwrap() {
                            HOST => {
                                let json: Value = serde_json::from_str(&line).unwrap();
                                let time = json.get("@timestamp").unwrap().as_str().unwrap();
                                let message = json.get("message").unwrap().as_str().unwrap();
                                if timestamp {
                                    vec.push((
                                        node.id.clone(),
                                        file.clone(),
                                        format!("{}: {}", time, message),
                                    ));
                                } else {
                                    vec.push((node.id.clone(), file.clone(), message.to_string()));
                                }
                            }
                            LogSourceTypes::SERIAL => {}
                        }
                    }
                }
            }
        }
    }
    vec
}

pub async fn watch_logs(
    node: &Node,
    tx: Sender<(String, String)>,
    watch: bool,
    all: bool,
    timestamp: bool,
) -> std::io::Result<()> {
    let mut lines = MuxedLines::new()?;
    let log_sources = get_log_sources_of(&node);
    let (files, file_to_types) = get_log_sources(log_sources);
    if all {
        for file in files {
            for line in get_lines_from_file(&file)? {
                match file_to_types.get(&file).unwrap() {
                    HOST => {
                        let json: Value = serde_json::from_str(&line)?;
                        let time = json.get("@timestamp").unwrap().as_str().unwrap();
                        let message = json.get("message").unwrap().as_str().unwrap();
                        if timestamp {
                            tx.send((file.clone(), format!("{}: {}", time, message)))
                                .await
                                .unwrap();
                        } else {
                            tx.send((file.clone(), message.to_string())).await.unwrap();
                        }
                    }
                    LogSourceTypes::SERIAL => {}
                }
            }
            lines.add_file(file).await?;
        }
    }
    if watch {
        while let Ok(Some(log)) = lines.next_line().await {
            let line = log.line();
            let file = log.source().display().to_string();
            match file_to_types.get(&file).unwrap() {
                HOST => {
                    let json: Value = serde_json::from_str(line)?;
                    let time = json.get("@timestamp").unwrap().as_str().unwrap();
                    let message = json.get("message").unwrap().as_str().unwrap();
                    if timestamp {
                        tx.send((file, format!("{}: {}", time, message)))
                            .await
                            .unwrap();
                    } else {
                        tx.send((file, message.to_string())).await.unwrap();
                    }
                }
                LogSourceTypes::SERIAL => {}
            }
        }
    }
    Ok(())
}
