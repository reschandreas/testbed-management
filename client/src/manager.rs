use colored::Colorize;
use prettytable::format;
use prettytable::{Cell, Row, Table};
use reqwest::Client;
use std::io::Write;
use std::path::Path;
use std::{env, fs};
use structs::deployment::Deployment;
use structs::deployment_row::DeploymentRow;
use structs::image_row::ImageRow;
use structs::node::Node;
use structs::node_row::NodeRow;
use structs::service_row::ServiceRow;
use yaml_rust::YamlLoader;

fn get_server_address() -> String {
    format!(
        "http://{}",
        env::var("CLUSTER_SERVER").unwrap_or_else(|_| String::from("localhost:9090"))
    )
}

pub async fn list_services(all: bool, group: bool) -> std::io::Result<()> {
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

    for row in get_service_rows(all, group).await.unwrap_or_default() {
        table.add_row(Row::new(row.get_cells()));
    }
    table.printstd();
    Ok(())
}

pub async fn get_service_rows(all: bool, group: bool) -> Result<Vec<ServiceRow>, reqwest::Error> {
    let resp = reqwest::get(format!(
        "{}/service/list/{}/{}",
        get_server_address(),
        all,
        group
    ))
    .await?
    .json::<Vec<ServiceRow>>()
    .await?;
    Ok(resp)
}

pub async fn list_nodes(all: bool) -> std::io::Result<()> {
    let mut table = Table::new();
    table.set_format(*format::consts::FORMAT_NO_BORDER_LINE_SEPARATOR);
    table.set_titles(Row::new(
        [
            Cell::new("id"),
            Cell::new("name"),
            Cell::new("MAC-Address"),
            Cell::new("TFTP-Prefix"),
            Cell::new("serial-number"),
            Cell::new("status"),
            Cell::new("hostname"),
            Cell::new("IPv4-address"),
            Cell::new("usable"),
        ]
        .to_vec(),
    ));
    match get_nodes_rows(all).await {
        Ok(rows) => {
            for row in rows {
                table.add_row(Row::new(row.get_cells()));
            }
        }
        Err(err) => {
            eprintln!("{:?}", err)
        }
    }
    table.printstd();
    Ok(())
}

pub async fn get_nodes_rows(all: bool) -> Result<Vec<NodeRow>, reqwest::Error> {
    let resp = reqwest::get(format!("{}/node/list/{}", get_server_address(), all))
        .await?
        .json::<Vec<NodeRow>>()
        .await?;
    Ok(resp)
}

pub async fn list_deployments(all: bool) -> std::io::Result<()> {
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

    for row in get_deployment_rows(all).await.unwrap() {
        table.add_row(Row::new(row.get_cells()));
    }
    table.printstd();
    Ok(())
}

pub async fn list_images() -> std::io::Result<()> {
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

    for row in get_images_rows().await.unwrap() {
        table.add_row(Row::new(row.get_cells()));
    }

    table.printstd();
    Ok(())
}

pub async fn get_deployment_rows(all: bool) -> Result<Vec<DeploymentRow>, reqwest::Error> {
    let resp = reqwest::get(format!("{}/deployment/list/{}", get_server_address(), all))
        .await?
        .json::<Vec<DeploymentRow>>()
        .await?;
    Ok(resp)
}

pub async fn get_node_by_id(id: &str, all: bool) -> Result<Node, reqwest::Error> {
    let resp = reqwest::get(format!("{}/node/get/{}/{}", get_server_address(), id, all))
        .await?
        .json::<Node>()
        .await?;
    Ok(resp)
}

pub async fn get_images_rows() -> Result<Vec<ImageRow>, reqwest::Error> {
    let resp = reqwest::get(format!("{}/image/list", get_server_address()))
        .await?
        .json::<Vec<ImageRow>>()
        .await?;
    Ok(resp)
}

pub async fn deploy_single_image(image: &str, node: Option<Node>) -> Result<bool, reqwest::Error> {
    let client = reqwest::Client::new();
    let resp = client
        .put(format!("{}/deploy/image", get_server_address()))
        .body(serde_json::to_string(&(image, node)).unwrap())
        .send()
        .await?
        .json::<bool>()
        .await?;
    Ok(resp)
}
pub async fn deploy_deployment(deployment_yaml: &str) -> Result<bool, reqwest::Error> {
    let path = Path::new(deployment_yaml);
    if !path.exists() {
        println!("{}", "please provide an existing yaml file".red());
        return Ok(false);
    }
    let content =
        &fs::read_to_string(deployment_yaml).expect("Something went wrong reading the file");
    let yaml = YamlLoader::load_from_str(content).unwrap();
    let deployment = Deployment::from_yaml(deployment_yaml, &yaml[0]);
    deploy(&deployment).await
}

async fn deploy(deployment: &Deployment) -> Result<bool, reqwest::Error> {
    let client = reqwest::Client::new();
    let resp = client
        .put(format!("{}/deploy/file", get_server_address()))
        .body(serde_json::to_string(deployment).unwrap())
        .send()
        .await?
        .json::<bool>()
        .await?;
    Ok(resp)
}

pub async fn get_deployment_logs(id: i64) -> bool {
    let client = Client::builder().build().unwrap();
    if let Ok(response) = client
        .get(format!("{}/deployment/logs/{}", get_server_address(), id))
        .send().await
    {
        let mut file = std::fs::File::create(format!("./{}.zip", id)).unwrap();
        return file.write_all(&response.bytes().await.unwrap()).is_ok();
    }
    false
}
