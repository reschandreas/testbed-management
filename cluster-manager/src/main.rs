use crate::config::get_node_by_id;
use crate::database::check;
use crate::deployer::deploy_single_image;
use crate::manager::list_images;
use clap::{App, Arg, ArgMatches};
use std::collections::HashMap;

mod config;
mod database;
mod deployer;
mod installer;
mod logs_manager;
mod manager;
mod node_manager;
mod power_manager;
mod server;
mod watcher;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    sudo::escalate_if_needed().unwrap();
    check();
    let matches = App::new("cluster-manager")
        .version("0.1")
        .author("Andreas Resch <andreas@resch.io>")
        .about("Manage your heterogeneous testbed with this collection of useful commands")
        .subcommand(add_check_subcommand())
        .subcommand(add_image_subcommand())
        .subcommand(add_install_subcommand())
        .subcommand(add_node_subcommand())
        .subcommand(add_deploy_subcommand())
        .subcommand(add_service_subcommand())
        .subcommand(add_deployment_subcommand())
        .subcommand(add_watch_subcommand())
        .subcommand(add_server_subcommand())
        .get_matches();

    handle_subcommands(&matches).await;
    Ok(())
}

fn add_check_subcommand() -> App<'static, 'static> {
    App::new("check").about("check required packages required by cluster-manager")
}

fn add_image_subcommand() -> App<'static, 'static> {
    App::new("image")
        .about("manage the operating system images")
        .subcommand(App::new("list").about("list the available images"))
}

fn add_install_subcommand() -> App<'static, 'static> {
    App::new("install").about("create required directories")
}

fn add_node_subcommand() -> App<'static, 'static> {
    App::new("node")
        .about("run commands to manage your nodes")
        .subcommand(
            App::new("list").about("list all configured nodes").arg(
                Arg::with_name("all")
                    .short("a")
                    .long("all")
                    .help("Show all nodes")
                    .takes_value(false),
            ),
        )
        .subcommand(
            App::new("add").about("add a new node to your cluster").arg(
                Arg::with_name("id")
                    .long("id")
                    .help("Configured identifier in config.yml")
                    .required(true)
                    .takes_value(true),
            ),
        )
        .subcommand(
            App::new("del")
                .about("remove a node from your cluster")
                .arg(
                    Arg::with_name("id")
                        .long("id")
                        .help("Configured identifier in config.yml")
                        .required(true)
                        .takes_value(true),
                ),
        )
        .subcommand(
            App::new("stop").about("stop a node").arg(
                Arg::with_name("id")
                    .long("id")
                    .help("Configured identifier in config.yml")
                    .required(true)
                    .takes_value(true),
            ),
        )
}

fn add_deploy_subcommand() -> App<'static, 'static> {
    App::new("deploy")
        .about("deploy images on the cluster")
        .arg(
            Arg::with_name("image")
                .short("i")
                .long("image")
                .help("Image which should be deployed")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("node")
                .short("n")
                .long("node")
                .help("Id of the node where the image should be deployed")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("file")
                .short("f")
                .long("file")
                .help("yaml file to deploy several nodes at once")
                .takes_value(true),
        )
}

fn add_service_subcommand() -> App<'static, 'static> {
    App::new("service")
        .about("run commands to manage your services")
        .subcommand(
            App::new("list")
                .about("list services in the cluster")
                .arg(
                    Arg::with_name("all")
                        .short("a")
                        .long("all")
                        .help("show all services, stopped included")
                        .takes_value(false),
                )
                .arg(
                    Arg::with_name("group")
                        .short("g")
                        .long("group")
                        .help("group services")
                        .takes_value(true),
                ),
        )
        .subcommand(
            App::new("stop")
                .about("stop service with the given id")
                .arg(
                    Arg::with_name("id")
                        .long("id")
                        .help("id for which service should be stopped")
                        .required(true)
                        .takes_value(true),
                )
                .arg(
                    Arg::with_name("prune")
                        .long("prune")
                        .help("prune sd_card of node")
                        .takes_value(false),
                ),
        )
}

fn add_deployment_subcommand() -> App<'static, 'static> {
    App::new("deployment")
        .about("run commands to manage your deployments")
        .subcommand(
            App::new("list")
                .about("list deployments in the cluster")
                .arg(
                    Arg::with_name("all")
                        .short("a")
                        .long("all")
                        .help("show all deployments, stopped included")
                        .takes_value(false),
                ),
        )
        .subcommand(
            App::new("stop")
                .about("stop deployments with the given id")
                .arg(
                    Arg::with_name("id")
                        .long("id")
                        .help("id for which deployments should be stopped")
                        .required(true)
                        .takes_value(true),
                )
                .arg(
                    Arg::with_name("prune")
                        .long("prune")
                        .help("prune sd_card of node")
                        .takes_value(false),
                ),
        )
}

fn add_watch_subcommand() -> App<'static, 'static> {
    App::new("watch")
        .about("watch logs of a node or a service")
        .subcommand(
            App::new("node").about("watch logs of the given node").arg(
                Arg::with_name("id")
                    .help("id for which node should be watched")
                    .required(true)
                    .takes_value(true),
            ),
        )
        .subcommand(
            App::new("service")
                .about("watch logs of the given service")
                .arg(
                    Arg::with_name("id")
                        .help("id for which service should be watched")
                        .required(true)
                        .takes_value(true),
                ),
        )
}

fn add_server_subcommand() -> App<'static, 'static> {
    App::new("server")
        .about("start server for remote management")
        .arg(
            Arg::with_name("ip-address")
                .help("address where the server should listen")
                .required(true)
                .takes_value(true),
        )
        .arg(
            Arg::with_name("port")
                .help("port on which the server should listen")
                .required(true)
                .takes_value(true),
        )
}

async fn handle_subcommands(matches: &ArgMatches<'_>) {
    let mut subcommands: HashMap<&str, fn(&ArgMatches)> = HashMap::new();
    subcommands.insert("install", handle_install_subcommand);
    subcommands.insert("check", handle_check_subcommand);
    subcommands.insert("node", handle_node_subcommand);
    subcommands.insert("deploy", handle_deploy_subcommand);
    subcommands.insert("service", handle_service_subcommand);
    subcommands.insert("deployment", handle_deployment_subcommand);
    subcommands.insert("image", handle_image_subcommand);
    for (command, function) in &subcommands {
        if let Some(matches) = matches.subcommand_matches(command) {
            function(matches);
            break;
        }
    }
    if let Some(matches) = matches.subcommand_matches("watch") {
        handle_watch_subcommand(&matches.clone()).await;
    }
    if let Some(matches) = matches.subcommand_matches("server") {
        server::start(
            matches.value_of("ip-address").unwrap().to_string(),
            matches.value_of("port").unwrap().to_string(),
        )
        .unwrap();
    }
}

fn handle_check_subcommand(_matches: &ArgMatches) {
    installer::check();
}

fn handle_install_subcommand(_matches: &ArgMatches) {
    installer::install();
}

fn handle_service_subcommand(matches: &ArgMatches) {
    if let Some(submatches) = matches.subcommand_matches("list") {
        self::manager::list_services(submatches.is_present("all"), submatches.is_present("group"));
    }
    if let Some(matches) = matches.subcommand_matches("stop") {
        if let Some(param) = matches.value_of("id") {
            if let Ok(id) = param.parse::<i64>() {
                self::manager::stop_service(id, matches.is_present("prune"));
            } else {
                eprintln!("Please provide a valid id")
            }
        } else {
            eprintln!("Please provide an id")
        }
    }
}

fn handle_deployment_subcommand(matches: &ArgMatches) {
    if let Some(submatches) = matches.subcommand_matches("list") {
        self::manager::list_deployments(submatches.is_present("all"));
    }
    if let Some(matches) = matches.subcommand_matches("stop") {
        if let Some(param) = matches.value_of("id") {
            if let Ok(id) = param.parse::<i64>() {
                self::manager::stop_deployment(id, matches.is_present("prune"));
            } else {
                eprintln!("Please provide a valid id")
            }
        } else {
            eprintln!("Please provide an id")
        }
    }
}

fn handle_node_subcommand(matches: &ArgMatches) {
    if let Some(submatches) = matches.subcommand_matches("list") {
        self::node_manager::list_nodes(submatches.is_present("all"));
    }
    if let Some(matches) = matches.subcommand_matches("add") {
        let identifier = parse_node_arguments(matches);
        self::node_manager::add_node(&identifier);
    }
    if let Some(matches) = matches.subcommand_matches("del") {
        if let Some(id) = matches.value_of("id") {
            if let Some(node) = get_node_by_id(id, false) {
                self::node_manager::remove_node(&node);
            } else {
                eprintln!("Please provide a valid id")
            }
        } else {
            eprintln!("Please provide an id")
        }
    }
    if let Some(matches) = matches.subcommand_matches("stop") {
        if let Some(id) = matches.value_of("id") {
            if let Some(node) = get_node_by_id(id, false) {
                self::node_manager::stop_node(&node, matches.is_present("prune"), false);
            } else {
                eprintln!("Please provide a valid id")
            }
        } else {
            eprintln!("Please provide an id")
        }
    }
}

fn handle_deploy_subcommand(matches: &ArgMatches) {
    if let Some(image) = matches.value_of("image") {
        let mut node = None;
        if let Some(id) = matches.value_of("node") {
            if let Some(n) = get_node_by_id(id, false) {
                node = Some(n);
            } else {
                eprintln!("Please provide a valid id");
                return;
            }
        }
        deploy_single_image(image, node);
    } else if let Some(file) = matches.value_of("file") {
        self::deployer::deploy_deployment(file);
    }
}

async fn handle_watch_subcommand(matches: &ArgMatches<'_>) {
    if let Some(matches) = matches.subcommand_matches("node") {
        if let Some(id) = matches.value_of("id") {
            self::manager::watch_node_logs_of(id).await.unwrap();
        } else {
            eprintln!("Please provide an id")
        }
    }
    if let Some(matches) = matches.subcommand_matches("service") {
        if let Some(param) = matches.value_of("id") {
            if let Ok(id) = param.parse::<i64>() {
                self::manager::watch_service_logs_of(id).await.unwrap();
            } else {
                eprintln!("Please provide a valid id")
            }
        } else {
            eprintln!("Please provide an id")
        }
    }
}

fn parse_node_arguments(matches: &ArgMatches) -> String {
    let prefix = matches
        .value_of("id")
        .expect("Please provide the configured identifier");
    prefix.to_string()
}

fn handle_image_subcommand(matches: &ArgMatches) {
    if let Some(_submatches) = matches.subcommand_matches("list") {
        list_images();
    }
}
