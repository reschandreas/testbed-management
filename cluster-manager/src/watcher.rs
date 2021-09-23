use crate::database::{get_running_deployments, get_services_by_deployment};
use crate::logs_manager::get_logs_of_deployment;
use crate::manager::stop_deployment;
use structs::deployment::Deployment;
use structs::task::Type::StopIfTrue;
use structs::task::{Task, Type};
use tokio::time::{sleep, Duration};

pub async fn watch() {
    loop {
        let deployments = get_running_deployments().unwrap();
        for mut deployment in deployments {
            deployment.services = get_services_by_deployment(deployment.id.unwrap()).unwrap();
            if !deployment.tasks.is_empty() {
                for task in &deployment.tasks {
                    if task.during_deployment {
                        match task.task_type {
                            StopIfTrue => {
                                if log_task_fulfilled(&task, &deployment).await {
                                    stop_deployment(deployment.id.unwrap(), false);
                                }
                            }
                            Type::NoOp
                            | Type::PurgeLocalStorage
                            | Type::DeleteLocalStorage
                            | Type::GetResults => {}
                        }
                    }
                }
            }
        }
        sleep(Duration::from_secs(60)).await;
    }
}

async fn log_task_fulfilled(task: &Task, deployment: &Deployment) -> bool {
    if let Ok((message, occurrence)) = serde_json::from_str::<(String, i64)>(&task.parameters) {
        let mut hits = 0;
        let logs = get_logs_of_deployment(deployment, false);
        for (_node, _filename, log) in logs {
            if message.eq(&log) {
                hits += 1;
            }
        }
        return hits >= occurrence;
    }
    false
}
