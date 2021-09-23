mod manager;
use crate::manager::{deploy_deployment, deploy_single_image, get_node_by_id};
use clap::{App, Arg, ArgMatches};

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let matches = App::new("client")
        .version("0.1")
        .author("Andreas Resch <andreas@resch.io>")
        .about("Manage your cluster remotely this collection of useful commands")
        .subcommand(add_node_subcommand())
        .subcommand(add_image_subcommand())
        .subcommand(add_service_subcommand())
        .subcommand(add_deployment_subcommand())
        .subcommand(add_deploy_subcommand())
        .get_matches();

    handle_subcommands(&matches).await;
    Ok(())
}

async fn handle_subcommands(matches: &ArgMatches<'_>) {
    if let Some(matches) = matches.subcommand_matches("service") {
        handle_service_subcommand(&matches.clone()).await;
    }
    if let Some(matches) = matches.subcommand_matches("node") {
        handle_node_subcommand(&matches.clone()).await;
    }
    if let Some(matches) = matches.subcommand_matches("deployment") {
        handle_deployment_subcommand(&matches.clone()).await;
    }
    if let Some(matches) = matches.subcommand_matches("image") {
        handle_image_subcommand(&matches.clone()).await;
    }
    if let Some(matches) = matches.subcommand_matches("deploy") {
        handle_deploy_subcommand(&matches.clone()).await;
    }
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

async fn handle_service_subcommand(matches: &ArgMatches<'_>) {
    if let Some(submatches) = matches.subcommand_matches("list") {
        self::manager::list_services(submatches.is_present("all"), submatches.is_present("group"))
            .await
            .unwrap();
    }
    /*if let Some(matches) = matches.subcommand_matches("stop") {
        if let Some(param) = matches.value_of("id") {
            if let Ok(id) = param.parse::<i64>() {
                self::manager::stop_service(id, matches.is_present("prune"));
            } else {
                eprintln!("Please provide a valid id")
            }
        } else {
            eprintln!("Please provide an id")
        }
    }*/
}

async fn handle_node_subcommand(matches: &ArgMatches<'_>) {
    if let Some(submatches) = matches.subcommand_matches("list") {
        self::manager::list_nodes(submatches.is_present("all"))
            .await
            .unwrap();
    }
}

async fn handle_deployment_subcommand(matches: &ArgMatches<'_>) {
    if let Some(submatches) = matches.subcommand_matches("list") {
        self::manager::list_deployments(submatches.is_present("all"))
            .await
            .unwrap();
    }

    if let Some(submatches) = matches.subcommand_matches("logs") {
        if let Some(param) = submatches.value_of("id") {
            if let Ok(id) = param.parse::<i64>() {
                self::manager::get_deployment_logs(id).await;
            } else {
                eprintln!("Please provide a valid id")
            }
        } else {
            eprintln!("Please provide an id")
        }
    }
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
            App::new("logs").about("retrieve logs of a deployment").arg(
                Arg::with_name("id")
                    .long("id")
                    .help("id for logs should be retrieved")
                    .required(true)
                    .takes_value(true),
            ),
        )
}

async fn handle_image_subcommand(matches: &ArgMatches<'_>) {
    if let Some(_submatches) = matches.subcommand_matches("list") {
        self::manager::list_images().await.unwrap();
    }
}

fn add_image_subcommand() -> App<'static, 'static> {
    App::new("image")
        .about("manage the operating system images")
        .subcommand(App::new("list").about("list the available images"))
}

async fn handle_deploy_subcommand(matches: &ArgMatches<'_>) {
    if let Some(image) = matches.value_of("image") {
        let mut node = None;
        if let Some(id) = matches.value_of("node") {
            if let Ok(n) = get_node_by_id(id, true).await {
                node = Some(n);
            } else {
                eprintln!("Please provide a valid id");
                return;
            }
        }
        println!("{}", deploy_single_image(image, node).await.unwrap());
    } else if let Some(file) = matches.value_of("file") {
        println!("{}", deploy_deployment(file).await.unwrap());
    }
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
