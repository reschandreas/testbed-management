use reqwest::blocking::Client;
use std::env;
use std::io::Write;
use std::process::Command;
use structs::utils::sha256sum_of_file;

fn get_server_address() -> String {
    format!(
        "http://{}",
        env::var("CLUSTER_SERVER").unwrap_or_else(|_| String::from("localhost:9090"))
    )
}

pub fn pull_image(name: &str, destination: &str) -> Result<(), reqwest::Error> {
    let client = Client::builder().timeout(None).build().unwrap();
    let response = client
        .get(format!("{}/image/download/{}", get_server_address(), name))
        .send()
        .unwrap();
    let mut file = std::fs::File::create(destination).unwrap();
    file.write_all(&response.bytes().unwrap()).unwrap();
    Ok(())
}

pub fn push_image(name: &str, filepath: &str) -> Result<bool, reqwest::Error> {
    let checksum = sha256sum_of_file(filepath).unwrap();
    let command = Command::new("curl")
        .arg(format!(
            "{}/image/upload/{}/{}",
            get_server_address(),
            format!("{}.zip", name),
            checksum
        ))
        .arg("-X")
        .arg("POST")
        .arg("-H")
        .arg("Content-Type: multipart/form-data")
        .arg("--form")
        .arg(format!("file=@{}", filepath))
        .spawn()
        .unwrap();
    let status = command.wait_with_output().unwrap();
    if status.status.success() {
        Ok(true)
    } else {
        let stdout = std::str::from_utf8(&status.stdout).unwrap();
        let stderr = std::str::from_utf8(&status.stderr).unwrap();
        eprintln!("{}", stdout);
        eprintln!("{}", stderr);
        Ok(false)
    }
}
