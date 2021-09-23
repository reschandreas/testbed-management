use crate::config::get_node_by_id;
use crate::deployer::{deploy, deploy_single_image};
use crate::installer::{MOVE, OS_IMAGES_DIR, RESULTS_DIR};
use crate::manager::{get_deployment_rows, get_images_rows, get_service_rows};
use crate::node_manager::get_nodes_rows;
use crate::watcher::watch;
use actix_multipart::Multipart;
use actix_web::body::Body;
use actix_web::http::StatusCode;
use actix_web::middleware::Logger;
use actix_web::{get, post, put, web, App, Error, HttpResponse, HttpServer, Responder};
use env_logger::Env;
use futures::StreamExt;
use std::fs;
use std::io::Write;
use std::path::Path;
use std::process::Command;
use structs::deployment::Deployment;
use structs::node::Node;
use structs::utils::sha256sum_matches;

#[get("/service/list/{active}/{group}")]
async fn list_services(web::Path((all, group)): web::Path<(bool, bool)>) -> impl Responder {
    let results = get_service_rows(all, group);
    serde_json::to_string(&results).unwrap()
}

#[get("/node/list/{all}")]
async fn list_nodes(web::Path(all): web::Path<bool>) -> impl Responder {
    let results = get_nodes_rows(all);
    serde_json::to_string(&results).unwrap()
}

#[get("/node/get/{id}/{all}")]
async fn get_node(web::Path((id, all)): web::Path<(String, bool)>) -> HttpResponse {
    match get_node_by_id(&id, all) {
        Some(node) => HttpResponse::new(StatusCode::OK)
            .set_body(Body::from(serde_json::to_string(&node).unwrap())),
        None => HttpResponse::new(StatusCode::NOT_FOUND).set_body(Body::Empty),
    }
}

#[get("/image/list")]
async fn list_images() -> impl Responder {
    let vec = get_images_rows();
    serde_json::to_string(&vec).unwrap()
}

#[put("/deploy/image")]
async fn deploy_image(body: web::Bytes) -> Result<HttpResponse, Error> {
    let result =
        serde_json::from_str::<(String, Option<Node>)>(std::str::from_utf8(&body).unwrap());
    match result {
        Ok((image, node)) => {
            let status = deploy_single_image(&image, node);
            Ok(HttpResponse::Ok()
                .content_type("application/json")
                .body(serde_json::to_string(&status).unwrap()))
        }
        Err(e) => {
            eprintln!("{}", e.to_string());
            Ok(HttpResponse::NotFound().body(Body::None))
        }
    }
}

#[put("/deploy/file")]
async fn deploy_file(body: web::Bytes) -> Result<HttpResponse, Error> {
    let result = serde_json::from_str::<Deployment>(std::str::from_utf8(&body).unwrap());
    match result {
        Ok(mut deployment) => {
            let status = deploy(&mut deployment);
            Ok(HttpResponse::Ok()
                .content_type("application/json")
                .body(serde_json::to_string(&status).unwrap()))
        }
        Err(e) => {
            eprintln!("{}", e.to_string());
            Ok(HttpResponse::NotFound().body(Body::None))
        }
    }
}

#[get("/deployment/list/{all}")]
async fn list_deployments(web::Path(all): web::Path<bool>) -> impl Responder {
    let results = get_deployment_rows(all);
    serde_json::to_string(&results).unwrap()
}

#[get("/deployment/logs/{id}")]
async fn get_deployment_logs(web::Path(id): web::Path<i64>) -> Result<HttpResponse, Error> {
    let filename = format!("{}.zip", id);
    let path = format!("{}/{}", RESULTS_DIR, filename);
    if !Path::new(path.as_str()).exists() {
        return Ok(HttpResponse::NotFound().body(Body::None));
    }
    let data = fs::read(path).unwrap();
    Ok(HttpResponse::Ok()
        .header(
            "Content-Disposition",
            format!("form-data; filename={}", filename),
        )
        .body(data))
}

const UPLOAD_PATH: &str = "/tmp/rest-api/upload";

#[post("/image/upload/{name}/{checksum}")]
async fn upload_image(
    web::Path((filename, checksum)): web::Path<(String, String)>,
    mut payload: Multipart,
) -> Result<HttpResponse, Error> {
    fs::create_dir_all(UPLOAD_PATH).unwrap();
    let filepath = format!("{}/{}", UPLOAD_PATH, &filename);
    while let Some(Ok(mut field)) = payload.next().await {
        // File::create is blocking operation, use thread pool
        let create_path = filepath.clone();
        match web::block(|| std::fs::File::create(create_path)).await {
            Ok(mut file) => {
                // Field in turn is stream of *Bytes* object
                while let Some(chunk) = field.next().await {
                    let data = chunk.unwrap();
                    // filesystem operations are blocking, we have to use thread pool
                    match web::block(move || file.write_all(&data).map(|_| file)).await {
                        Ok(updated_file) => {
                            file = updated_file;
                        }
                        Err(e) => {
                            eprintln!("{:?}", e);
                            return Ok(HttpResponse::Conflict().body("error on file creation"));
                        }
                    }
                }
            }
            Err(e) => {
                eprintln!("{:?}", e);
                return Ok(HttpResponse::Conflict().body("error getting file"));
            }
        }
    }
    return if sha256sum_matches(&format!("{}/{}", UPLOAD_PATH, &filename), &checksum) {
        let move_path = filepath.clone();
        return match web::block(move || {
            Command::new(MOVE)
                .arg(&move_path)
                .arg(format!("{}/{}", OS_IMAGES_DIR, filename))
                .spawn()
                .unwrap()
                .wait()
        })
        .await
        {
            Ok(status) => {
                if status.success() {
                    Ok(HttpResponse::Ok().json(&true))
                } else {
                    Ok(HttpResponse::Conflict().body(Body::None))
                }
            }
            Err(e) => {
                eprintln!("{:?}", e);
                Ok(HttpResponse::Conflict().body("error on file creation"))
            }
        };
    } else {
        let delete_path = filepath.clone();
        web::block(move || fs::remove_file(&delete_path))
            .await
            .unwrap();
        Ok(HttpResponse::Conflict().body(Body::None))
    };
}

#[get("/image/download/{name}")]
async fn download_image(web::Path(name): web::Path<String>) -> Result<HttpResponse, Error> {
    let filename = format!("{}.zip", name);
    let path = format!("{}/{}", OS_IMAGES_DIR, filename);
    if !Path::new(path.as_str()).exists() {
        return Ok(HttpResponse::NotFound().body(Body::None));
    }
    let data = fs::read(path).unwrap();
    Ok(HttpResponse::Ok()
        .header(
            "Content-Disposition",
            format!("form-data; filename={}", filename),
        )
        .body(data))
}

#[actix_web::main]
pub(crate) async fn start(ip_address: String, port: String) -> std::io::Result<()> {
    let addr = format!("{}:{}", ip_address, port);
    let _handle = tokio::spawn(async move { watch().await });
    println!("Listening on {}", addr);
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();
    HttpServer::new(|| {
        App::new()
            .wrap(Logger::default())
            .service(list_services)
            .service(list_nodes)
            .service(get_node)
            .service(list_deployments)
            .service(list_images)
            .service(deploy_image)
            .service(deploy_file)
            .service(upload_image)
            .service(download_image)
            .service(get_deployment_logs)
    })
    .bind(addr)?
    .run()
    .await
}
