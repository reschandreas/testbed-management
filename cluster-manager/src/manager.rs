use crate::config::get_node_by_id;
use crate::database::{
    get_deployment_by_id, get_deployments, get_running_deployments, get_running_services,
    get_service_by_id, get_services, get_services_by_deployment, get_tasks_by_deployment,
    set_enddate_for_deployment, set_enddate_for_service,
};
use crate::deployer::{extract_configuration, retrieve_local_logs};
use crate::installer::OS_IMAGES_DIR;
use crate::logs_manager::{collect_deployment_logs, watch_logs};
use crate::node_manager::stop_node;
use colored::Colorize;
use prettytable::format;
use prettytable::{Cell, Row, Table};
use std::sync::mpsc;
use std::sync::mpsc::Sender;
use std::thread::JoinHandle;
use std::{fs, thread};
use structs::deployment::Deployment;
use structs::deployment_row::DeploymentRow;
use structs::image_row::ImageRow;
use structs::mountpoint::Mountpoint;
use structs::node::Node;
use structs::service::Service;
use structs::service_row::ServiceRow;
use structs::task::Task;
use structs::task::Type::GetResults;
use structs::utils::print_message;

pub fn list_services(all: bool, group: bool) {
    let mut table = Table::new();
    table.set_format(*format::consts::FORMAT_NO_BORDER_LINE_SEPARATOR);
    table.set_titles(Row::new(
        [
            Cell::new("id"),
            Cell::new("name"),
            Cell::new("image"),
            Cell::new("node"),
            Cell::new("deployment"),
            Cell::new("hostname"),
            Cell::new("IPv4-Address"),
            Cell::new("started"),
            Cell::new("ended"),
            Cell::new("replicas"),
        ]
        .to_vec(),
    ));

    for row in get_service_rows(all, group) {
        table.add_row(Row::new(row.get_cells()));
    }
    table.printstd();
}

pub fn get_service_rows(all: bool, group: bool) -> Vec<ServiceRow> {
    let mut rows = Vec::new();
    if let Ok(mut services) = if all {
        get_services()
    } else {
        get_running_services()
    } {
        if services.is_empty() {
            return Vec::new();
        }
        if group {
            services = Service::group_services(services);
        }
        let mut handles: Vec<JoinHandle<()>> = Vec::new();
        let (tx, rx) = mpsc::channel();
        for service in services {
            handles.push(get_service_line_handle(service.clone(), tx.clone()));
        }
        for handle in handles {
            handle.join().unwrap();
            if let Some(row) = rx.recv().unwrap() {
                rows.push(row);
            }
        }
    } else {
        eprintln!("Problem with reading services from database");
    }
    rows
}

pub fn stop_service(id: i64, prune: bool) {
    if let Ok(service) = get_service_by_id(id, true) {
        let node = get_node_by_id(&service.node.unwrap(), false).unwrap();
        let stopped = stop_node(&node, prune, false);
        print_message("stop node", stopped);
        if stopped {
            print_message("stop service", set_enddate_for_service(id).is_ok());
        }
    } else {
        eprintln!("No such service found");
    }
}

pub fn stop_deployment(id: i64, prune: bool) {
    if let Ok(mut deployment) = get_deployment_by_id(id, true) {
        let tasks = match get_tasks_by_deployment(id) {
            Ok(t) => t,
            _ => Vec::new(),
        };
        if let Ok(services) = get_services_by_deployment(deployment.id.unwrap()) {
            for service in &services {
                for task in tasks
                    .iter()
                    .filter_map(|t| {
                        if t.task_type.eq(&GetResults)
                            && t.service.as_ref().unwrap().id.unwrap() == service.id.unwrap()
                        {
                            return Some(t.to_owned());
                        }
                        None
                    })
                    .collect::<Vec<Task>>()
                {
                    let mountpoint: Mountpoint = serde_json::from_str(&task.parameters).unwrap();
                    let node = get_node_by_id(&service.node.as_ref().unwrap(), false).unwrap();
                    retrieve_local_logs(&mut deployment, &service, &node, &mountpoint);
                }
                stop_service(service.id.unwrap(), prune);
            }
            collect_deployment_logs(id, services);
            print_message("stop deployment", set_enddate_for_deployment(id).is_ok());
        } else {
            eprintln!("No services for this deployment found");
        }
    } else {
        eprintln!("No such deployment found");
    }
}

pub fn list_deployments(all: bool) {
    let mut table = Table::new();
    table.set_format(*format::consts::FORMAT_NO_BORDER_LINE_SEPARATOR);
    table.set_titles(Row::new(
        [
            Cell::new("id"),
            Cell::new("name"),
            Cell::new("start"),
            Cell::new("end"),
            Cell::new("owner"),
            Cell::new("#services"),
        ]
        .to_vec(),
    ));

    for row in get_deployment_rows(all) {
        table.add_row(Row::new(row.get_cells()));
    }

    table.printstd()
}

pub fn get_deployment_rows(all: bool) -> Vec<DeploymentRow> {
    let mut rows = Vec::new();
    if let Ok(deployments) = if all {
        get_deployments()
    } else {
        get_running_deployments()
    } {
        if deployments.is_empty() {
            return Vec::new();
        }
        let mut handles: Vec<JoinHandle<()>> = Vec::new();
        let (tx, rx) = mpsc::channel();
        for deployment in deployments {
            handles.push(get_deployment_line_handle(deployment.clone(), tx.clone()));
        }
        for handle in handles {
            handle.join().unwrap();
            let row = rx.recv().unwrap();
            rows.push(row);
        }
    } else {
        eprintln!("Problem with reading deployments from database");
    }
    rows
}

fn get_service_line_handle(service: Service, tx: Sender<Option<ServiceRow>>) -> JoinHandle<()> {
    thread::spawn(move || {
        get_node_by_id(&service.clone().node.unwrap(), true).map_or_else(
            || tx.send(None).unwrap(),
            |node| {
                let deployment = get_deployment_by_id(service.deployment.unwrap(), false)
                    .unwrap()
                    .name;
                let row = ServiceRow::new(&service, node.name, deployment);
                tx.send(Some(row)).unwrap()
            },
        );
    })
}

fn get_deployment_line_handle(deployment: Deployment, tx: Sender<DeploymentRow>) -> JoinHandle<()> {
    thread::spawn(move || {
        let services = match get_services_by_deployment(deployment.id.unwrap()) {
            Ok(vec) => Some(vec.len()),
            Err(_) => None,
        };
        let row = DeploymentRow::new(deployment, services);
        tx.send(row).unwrap()
    })
}

pub async fn watch_service_logs_of(id: i64) -> std::io::Result<()> {
    if let Ok(service) = get_service_by_id(id, true) {
        let node = get_node_by_id(&service.node.unwrap(), false).unwrap();
        if watch_logs_of(node, true, true, true).await.is_err() {
            eprintln!("{}", format!("{}", "No logs available".red()));
        }
    } else {
        eprintln!("No such service found");
    }
    Ok(())
}

pub async fn watch_node_logs_of(id: &str) -> std::io::Result<()> {
    if let Some(node) = get_node_by_id(id, true) {
        if watch_logs_of(node, true, true, true).await.is_err() {
            eprintln!("{}", format!("{}", "No logs available".red()));
        }
    } else {
        eprintln!("No such node found");
    }
    Ok(())
}

async fn watch_logs_of(node: Node, watch: bool, all: bool, filenames: bool) -> std::io::Result<()> {
    use tokio::sync::mpsc;
    let (tx, mut rx) = mpsc::channel(1);
    let _handle = tokio::spawn(async move { watch_logs(&node, tx, watch, all, true).await });
    let mut last_file = String::new();
    loop {
        if let Some((file, message)) = rx.recv().await {
            if !last_file.eq(&file) {
                if filenames {
                    println!("==> {} <==", file);
                }
                last_file = file.clone();
            }
            println!("{}", message);
        }
    }
}

pub fn list_images() {
    let mut table = Table::new();
    table.set_format(*format::consts::FORMAT_NO_BORDER_LINE_SEPARATOR);
    table.set_titles(Row::new(
        [
            Cell::new("name"),
            Cell::new("architecture"),
            Cell::new("on-device"),
        ]
        .to_vec(),
    ));

    for row in get_images_rows() {
        table.add_row(Row::new(row.get_cells()));
    }

    table.printstd()
}

pub fn get_images_rows() -> Vec<ImageRow> {
    let mut vec = Vec::new();
    if let Ok(files) = fs::read_dir(OS_IMAGES_DIR) {
        for file in files {
            if let Ok(entry) = file {
                let filename = String::from(entry.file_name().to_str().unwrap());
                let ending = filename.split('.').last().unwrap().to_string();
                if ending.eq("zip") {
                    let name = filename.replace(".zip", "");
                    let configuration = extract_configuration(&name);
                    let row = ImageRow::new(name, configuration);
                    vec.push(row);
                }
            }
        }
    }
    vec
}
