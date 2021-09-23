use crate::config::get_nodes;
use crate::installer::BASE_DIR;
use rand::prelude::SliceRandom;
use rand::thread_rng;
use rusqlite::{params, Connection, Error, Result};
use std::collections::HashMap;
use std::ops::Add;
use structs::deployment::Deployment;
use structs::node::Node;
use structs::service::Service;
use structs::task::Task;

#[derive(Debug)]
struct Person {
    id: i32,
    name: String,
    data: Option<Vec<u8>>,
}

fn get_connection() -> Result<Connection, Error> {
    Connection::open(format!("{}/{}", BASE_DIR, "cluster-manager.db"))
}

pub fn check() -> bool {
    setup()
}

fn setup() -> bool {
    if let Ok(conn) = get_connection() {
        conn.execute(
            "CREATE TABLE IF NOT EXISTS deployments  (
                  id              INTEGER PRIMARY KEY AUTOINCREMENT,
                  name            VARCHAR2(20) NOT NULL,
                  owner           VARCHAR2(20) NOT NULL,
                  start           DATETIME DEFAULT CURRENT_TIMESTAMP,
                  end             DATETIME
                  )",
            [],
        )
        .unwrap();
        conn.execute(
            "CREATE TABLE IF NOT EXISTS services (
                  id              INTEGER PRIMARY KEY AUTOINCREMENT,
                  name            VARCHAR2(20) NOT NULL,
                  image           VARCHAR2(20) NOT NULL,
                  deployment      INTEGER NOT NULL,
                  node            VARCHAR2(20) NOT NULL,
                  start           DATETIME DEFAULT CURRENT_TIMESTAMP,
                  end             DATETIME,
                  ipv4_address    VARCHAR(15),
                  hostname        VARCHAR(100) NOT NULL,
                  architecture    VARCHAR(20),
                  FOREIGN KEY(deployment) REFERENCES deployments(id),
                  FOREIGN KEY(node) REFERENCES nodes(id)
                  )",
            [],
        )
        .unwrap();
        conn.execute(
            "CREATE TABLE IF NOT EXISTS logs (
                timestamp       DATETIME DEFAULT CURRENT_TIMESTAMP,
                message         TEXT NOT NULL
                )",
            [],
        )
        .unwrap();
        conn.execute(
            "CREATE TABLE IF NOT EXISTS tasks  (
                  id                INTEGER PRIMARY KEY AUTOINCREMENT,
                  deployment        INTEGER NOT NULL,
                  service           INTEGER,
                  type              INTEGER NOT NULL,
                  parameters        VARCHAR2(200) NOT NULL,
                  during_deployment BOOLEAN NOT NULL CHECK (during_deployment IN (0, 1)),
                  start             DATETIME DEFAULT NULL,
                  end               DATETIME DEFAULT NULL,
                  FOREIGN KEY(deployment) REFERENCES deployments(id),
                  FOREIGN KEY(service) REFERENCES services(id)
                  )",
            [],
        )
        .unwrap();
        return true;
    }
    false
}

pub fn insert_deployment(deployment: &Deployment) -> Result<i64, Error> {
    let connection = get_connection()?;
    let mut stmt = connection.prepare("INSERT INTO deployments (name, owner) VALUES (?1, ?2)")?;
    let result = stmt.insert(params![deployment.name, deployment.owner]);
    if let Ok(id) = result {
        for task in &deployment.tasks {
            insert_task(task, id).unwrap();
        }
    }
    result
}

pub fn insert_service(service: &Service) -> Result<i64, Error> {
    let connection = get_connection()?;
    let mut stmt = connection.prepare(
            "INSERT INTO services (name, image, deployment, node, ipv4_address, hostname, architecture) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)")?;
    let arch = match &service.architecture {
        Some(a) => Some(a.get_name()),
        None => None,
    };
    stmt.insert(params![
        service.name,
        service.image,
        service.deployment.unwrap(),
        service.node.as_ref().unwrap(),
        service.ipv4_address.as_ref(),
        service.hostname,
        arch.unwrap(),
    ])
}

pub fn insert_task(task: &Task, deployment_id: i64) -> Result<i64, Error> {
    let connection = get_connection()?;
    return if task.service.is_some() {
        let mut stmt = connection.prepare(
            "INSERT INTO tasks (deployment, service, type, parameters, during_deployment) VALUES (?1, ?2, ?3, ?4, ?5)",
        )?;
        stmt.insert(params![
            deployment_id,
            task.service.as_ref().unwrap().id.unwrap(),
            task.task_type as usize,
            task.parameters,
            if task.during_deployment { 1 } else { 0 },
        ])
    } else {
        let mut stmt = connection
            .prepare("INSERT INTO tasks (deployment, type, parameters, during_deployment) VALUES (?1, ?2, ?3, ?4)")?;
        stmt.insert(params![
            deployment_id,
            task.task_type as usize,
            task.parameters,
            if task.during_deployment { 1 } else { 0 },
        ])
    };
}

pub fn get_idle_nodes() -> Result<Vec<Node>, Error> {
    let mut vec = Vec::new();
    let mut idle_nodes = HashMap::new();
    if let Ok(nodes) = get_nodes() {
        for node in nodes {
            idle_nodes.insert(node.id.clone(), node);
        }
        for service in get_running_services().unwrap() {
            idle_nodes.remove(&service.node.unwrap());
        }
        for (_key, node) in idle_nodes {
            vec.push(node.to_owned());
        }
    }
    vec.shuffle(&mut thread_rng());
    Ok(vec)
}

pub fn get_running_services() -> Result<Vec<Service>, Error> {
    let mut vec = Vec::new();
    let connection = get_connection()?;
    let mut stmt = connection.prepare("SELECT * FROM services s WHERE s.end IS NULL")?;
    let node_iter = stmt.query_map([], |row| Ok(Service::from_row(row)))?;
    node_iter
        .filter(std::result::Result::is_ok)
        .for_each(|s| vec.push(s.unwrap()));
    Ok(vec)
}

#[allow(dead_code)]
pub fn get_stopped_services() -> Result<Vec<Service>, Error> {
    let mut vec = Vec::new();
    let connection = get_connection()?;
    let mut stmt = connection.prepare("SELECT * FROM services s WHERE s.end IS NOT NULL")?;
    let node_iter = stmt.query_map([], |row| Ok(Service::from_row(row)))?;
    node_iter
        .filter(std::result::Result::is_ok)
        .for_each(|s| vec.push(s.unwrap()));
    Ok(vec)
}

pub fn get_services() -> Result<Vec<Service>, Error> {
    let mut vec = Vec::new();
    let connection = get_connection()?;
    let mut stmt = connection.prepare("SELECT * FROM services s")?;
    let node_iter = stmt.query_map([], |row| Ok(Service::from_row(row)))?;
    node_iter
        .filter(std::result::Result::is_ok)
        .for_each(|s| vec.push(s.unwrap()));
    Ok(vec)
}

pub fn get_deployment_by_id(id: i64, only_active: bool) -> Result<Deployment, Error> {
    let connection = get_connection()?;
    let mut query = String::from("SELECT * FROM deployments d WHERE d.id = ?1");
    if only_active {
        query = query.add(" AND d.end IS NULL");
    }
    let mut statement = connection.prepare(&query)?;
    let result = statement.query_row(params![id], |row| Ok(Deployment::from_row(row)))?;
    Ok(result)
}

pub fn get_service_by_id(id: i64, only_active: bool) -> Result<Service, Error> {
    let connection = get_connection()?;
    let mut query = String::from("SELECT * FROM services s WHERE s.id = ?1");
    if only_active {
        query = query.add(" AND s.end IS NULL");
    }
    let mut statement = connection.prepare(&query)?;
    let result = statement.query_row(params![id], |row| Ok(Service::from_row(row)))?;
    Ok(result)
}

pub fn set_enddate_for_service(id: i64) -> Result<usize, Error> {
    let connection = get_connection()?;
    let mut statement =
        connection.prepare("UPDATE services SET end = CURRENT_TIMESTAMP WHERE id = ?1")?;
    statement.execute(params![id])
}

pub fn get_deployments() -> Result<Vec<Deployment>, Error> {
    let mut vec = Vec::new();
    let connection = get_connection()?;
    let mut stmt = connection.prepare("SELECT * FROM deployments d")?;
    let node_iter = stmt.query_map([], |row| Ok(Deployment::from_row(row)))?;
    node_iter
        .filter(std::result::Result::is_ok)
        .for_each(|s| vec.push(s.unwrap()));
    Ok(vec)
}

pub fn get_running_deployments() -> Result<Vec<Deployment>, Error> {
    let mut vec = Vec::new();
    let connection = get_connection()?;
    let mut stmt = connection.prepare("SELECT * FROM deployments d WHERE d.end IS NULL")?;
    let node_iter = stmt.query_map([], |row| Ok(Deployment::from_row(row)))?;
    node_iter
        .filter(std::result::Result::is_ok)
        .for_each(|s| vec.push(s.unwrap()));
    let mut with_tasks = Vec::new();
    for mut deployment in vec {
        deployment.tasks = get_tasks_by_deployment(deployment.id.unwrap()).unwrap();
        with_tasks.push(deployment);
    }
    Ok(with_tasks)
}

pub fn get_services_by_deployment(id: i64) -> Result<Vec<Service>, Error> {
    let mut vec = Vec::new();
    let connection = get_connection()?;
    let mut stmt = connection.prepare("SELECT * FROM services s WHERE s.deployment = ?1")?;
    let node_iter = stmt.query_map([id], |row| Ok(Service::from_row(row)))?;
    node_iter
        .filter(std::result::Result::is_ok)
        .for_each(|s| vec.push(s.unwrap()));
    Ok(vec)
}

pub fn set_enddate_for_deployment(id: i64) -> Result<usize, Error> {
    let connection = get_connection()?;
    let mut statement =
        connection.prepare("UPDATE deployments SET end = CURRENT_TIMESTAMP WHERE id = ?1")?;
    statement.execute(params![id])
}

pub fn get_tasks_by_deployment(id: i64) -> Result<Vec<Task>, Error> {
    let mut vec = Vec::new();
    let connection = get_connection()?;
    let query = String::from("SELECT * FROM tasks t WHERE t.deployment = ?1");
    let mut stmt = connection.prepare(&query)?;
    let iter = stmt.query_map([id], |row| {
        let deployment = get_deployment_by_id(row.get(1).unwrap(), false).unwrap();
        let service = match row.get(2) {
            Ok(service_id) => Some(get_service_by_id(service_id, false).unwrap()),
            Err(_) => None,
        };
        Ok(Task::from_row(deployment, service, row))
    })?;
    iter.filter(std::result::Result::is_ok)
        .for_each(|s| vec.push(s.unwrap()));
    Ok(vec)
}

pub fn set_end_and_executed_for_task(id: i64) -> Result<usize, Error> {
    let connection = get_connection()?;
    let mut statement = connection
        .prepare("UPDATE tasks SET end = CURRENT_TIMESTAMP, executed = 1 WHERE id = ?1")?;
    statement.execute(params![id])
}
