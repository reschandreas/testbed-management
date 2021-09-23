use crate::client;
use colored::Colorize;
use std::collections::HashMap;
use std::fs;
use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};
use std::{io, str};
use structs::architecture::Architecture;
use structs::bootconfig::group;
use structs::configuration::Configuration;
use structs::imagefile::Imagefile;
use structs::mountpoint::{get_mount_order, Mountpoint};
use structs::provisioner::Types::{FILE, SHELL};
use structs::utils::{get_random_name, print_message, sha256sum_of_file};
use url::Url;

const BUILD_DIRECTORY: &str = "os-build";
const BASEIMAGE_DIRECTORY: &str = "base";

pub fn build(imagefile: &mut Imagefile, filename: &str, tag: &str) -> bool {
    print_message(
        "clean and create build environment",
        create_build_directory().is_ok(),
    );
    imagefile.configuration.name = tag.to_string();
    if imagefile.configuration.prebuilt {
        print_message(
            "move prebuilt image to sandbox",
            move_prebuilt_image_to_sandbox(&imagefile),
        );
        return complete_build(imagefile, tag);
    } else if let Ok(mut base_mountorder) = get_mountpoints_from_baseimage(imagefile) {
        move_files_to_sandbox(imagefile);
        write_pkr_hcl(imagefile, filename);
        print_message(
            "move preseed file if required",
            move_preseed_file(imagefile),
        );
        let packer = execute_packer(imagefile, filename);
        print_message("creating image with packer", packer.is_ok());
        if let Ok(mountorder) = packer {
            for mountpoint in mountorder {
                base_mountorder.insert(mountpoint.get_path(), mountpoint);
            }
            if !imagefile.configuration.mountorder_to_vec(base_mountorder) {
                eprintln!(
                    "{}",
                    "no mountpoints detected, does not seem right, check and rerun building process".red()
                );
            }
            return complete_build(imagefile, tag);
        }
    }
    false
}

fn complete_build(imagefile: &mut Imagefile, tag: &str) -> bool {
    print_message("writing configuration", write_configuration(imagefile));
    cleanup();
    print_message("compress image", zip_image(&imagefile, tag));
    remove_build_directory().is_ok()
}

fn cleanup() {
    let base_image = format!("./{}/{}", BUILD_DIRECTORY, BASEIMAGE_DIRECTORY);
    let base_image_path = Path::new(&base_image);
    if base_image_path.exists() {
        print_message("remove base image", fs::remove_dir_all(&base_image).is_ok());
    }
    let packer_cache = format!("./{}/{}", BUILD_DIRECTORY, "packer_cache");
    let packer_cache_path = Path::new(&packer_cache);
    if packer_cache_path.exists() {
        print_message(
            "remove packer cache",
            fs::remove_dir_all(packer_cache).is_ok(),
        );
    }
}

fn create_build_directory() -> io::Result<()> {
    if remove_build_directory().is_err() {
        eprintln!("Could not remove previous build directory");
    }
    fs::create_dir(Path::new(format!("./{}", BUILD_DIRECTORY).as_str()))
}

fn remove_build_directory() -> io::Result<()> {
    fs::remove_dir_all(Path::new(format!("./{}", BUILD_DIRECTORY).as_str()))
}

fn write_pkr_hcl(imagefile: &mut Imagefile, filename: &str) {
    let content = imagefile.as_pkr_hcl();
    let mut file = File::create(format!("./{}/{}", BUILD_DIRECTORY, filename)).unwrap();
    file.write_all(content.as_bytes()).unwrap();
}

fn execute_packer(imagefile: &Imagefile, file: &str) -> Result<Vec<Mountpoint>, String> {
    let output = match &imagefile.architecture {
        Architecture::ARM32 | Architecture::ARM64 => docker_packer(file),
        Architecture::X86 => native_packer(file),
    };
    match output {
        Ok(stdout) => Ok(get_mount_order(stdout.as_str())),
        Err(stderr) => Err(stderr),
    }
}

fn docker_packer(file: &str) -> Result<String, String> {
    let pwd_vec = Command::new("pwd").output().unwrap().stdout;
    let pwd = str::from_utf8(&pwd_vec).unwrap().replace('\n', "");
    let current_directory = pwd;
    let random_name = get_random_name();
    let mut child = Command::new("docker")
        .arg("run")
        .arg("--rm")
        .arg("--name")
        .arg(random_name.as_str())
        .arg("--privileged")
        .arg("-v")
        .arg("/dev:/dev")
        .arg("-v")
        .arg(format!("{}/{}:/build", current_directory, BUILD_DIRECTORY))
        .arg("mkaczanowski/packer-builder-arm")
        .arg("build")
        .arg(file)
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()
        .expect("failed to build image");
    let stdout;
    loop {
        let output = Command::new("docker")
            .arg("logs")
            .arg("-f")
            .arg(random_name.as_str())
            .output()
            .expect("could not read log of started container");
        if output.status.success() {
            let childs_stdout = String::from(str::from_utf8(&output.stdout).unwrap());
            let stderr = String::from(str::from_utf8(&output.stderr).unwrap());
            if !childs_stdout.is_empty() {
                stdout = String::from(str::from_utf8(&output.stdout).unwrap());
                break;
            }
            if !stderr.is_empty() {
                eprintln!("{}", stderr)
            }
        } else {
            eprintln!("Tried to get docker output")
        }
    }
    if child.wait().unwrap().success() && !stdout.is_empty() {
        return Ok(stdout);
    }
    Err("Could not execute docker command, check if docker is installed and running".to_string())
}

fn native_packer(file: &str) -> Result<String, String> {
    let pwd_vec = Command::new("pwd").output().unwrap().stdout;
    let pwd = str::from_utf8(&pwd_vec).unwrap().replace('\n', "");
    let child = Command::new("packer")
        .current_dir(format!("{}/{}/", pwd, BUILD_DIRECTORY))
        .arg("build")
        .arg(format!("{}/{}/{}", pwd, BUILD_DIRECTORY, file))
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()
        .expect("failed to build image");
    let output = child.wait_with_output().unwrap();
    if output.status.success() {
        let childs_stdout = String::from(str::from_utf8(&output.stdout).unwrap());
        Ok(childs_stdout)
    } else {
        let childs_stderr = String::from(str::from_utf8(&output.stderr).unwrap());
        Err(childs_stderr)
    }
}

fn zip_image(_imagefile: &Imagefile, tag: &str) -> bool {
    let mut child = Command::new("zip")
        .arg("-r")
        .arg(format!("{}.zip", tag))
        .arg(format!("./{}", BUILD_DIRECTORY))
        .spawn()
        .expect("failed to zip image");
    child.wait().unwrap().success()
}

fn get_base_image_from_repo(image: &str) -> bool {
    match client::pull_image(
        image,
        &format!(
            "./{}/{}/{}.zip",
            BUILD_DIRECTORY, BASEIMAGE_DIRECTORY, image
        ),
    ) {
        Ok(()) => true,
        Err(_) => false,
    }
}

fn unpack_base_image(image: &str, sandbox: &str) {
    print_message(
        "unpack base image in sandbox",
        unpack_image(
            format!(
                "./{}/{}/{}.zip",
                BUILD_DIRECTORY, BASEIMAGE_DIRECTORY, image
            )
            .as_str(),
            sandbox,
        ),
    );
}

fn get_mountpoints_from_baseimage(
    imagefile: &mut Imagefile,
) -> Result<HashMap<String, Mountpoint>, &str> {
    let image = String::from(imagefile.preamble.get_filename());
    let path = Path::new(image.as_str());
    fs::create_dir(format!("./{}/{}", BUILD_DIRECTORY, BASEIMAGE_DIRECTORY)).unwrap();
    if path.exists() {
        move_base_image_to_sandbox(image.as_str(), imagefile);
    } else {
        return if let Ok(_url) = Url::parse(image.as_str()) {
            Ok(HashMap::new())
        } else {
            handle_repo_base_image(image.as_str(), imagefile)
        };
    }
    Ok(HashMap::new())
}

fn handle_repo_base_image(
    image: &str,
    imagefile: &mut Imagefile,
) -> Result<HashMap<String, Mountpoint>, &'static str> {
    let status = get_base_image_from_repo(image);
    print_message("pulling image from server", status);
    return if status {
        let sandbox = create_baseimage_sandbox();
        unpack_base_image(image, sandbox.as_str());
        let configuration = read_configuration_of_base_image(&sandbox);
        print_message("read configuration", configuration.is_some());
        imagefile.configuration.merge(configuration.unwrap());
        if imagefile.configuration.mountorder.is_empty() {
            Err("No Mountorder found")
        } else {
            print_message(
                "generate sha256",
                generate_sha256(
                    format!(
                        "./{}/{}/{}/{}/generated.img",
                        BUILD_DIRECTORY, BASEIMAGE_DIRECTORY, sandbox, BUILD_DIRECTORY
                    )
                    .as_str(),
                ),
            );
            imagefile.preamble.set_filepath(
                format!(
                    "./{}/{}/{}/generated.img",
                    BASEIMAGE_DIRECTORY, sandbox, BUILD_DIRECTORY
                )
                .as_str(),
            );
            let mut hash = HashMap::new();
            for mountpoint in &imagefile.configuration.mountorder {
                hash.insert(mountpoint.get_path(), mountpoint.clone());
            }
            Ok(hash)
        }
    } else {
        Err("Could not unpack base image")
    };
}

fn move_base_image_to_sandbox(image: &str, imagefile: &mut Imagefile) {
    let imagepath = format!("./{}/{}", BUILD_DIRECTORY, BASEIMAGE_DIRECTORY);
    Command::new("cp")
        .arg(image)
        .arg(&imagepath)
        .spawn()
        .unwrap();
    let checksum = format!("{}.{}", image, imagefile.preamble.get_checksum_type());
    if Path::new(checksum.as_str()).exists() {
        Command::new("cp")
            .arg(checksum)
            .arg(&imagepath)
            .spawn()
            .unwrap();
    }
    imagefile
        .preamble
        .set_filepath(format!("./{}/{}", BASEIMAGE_DIRECTORY, image).as_str());
}

fn create_baseimage_sandbox() -> String {
    let sandbox_name = structs::utils::get_random_name();
    print_message(
        "create base image sandbox",
        create_base_image_tmp_directory(sandbox_name.as_str()).is_ok(),
    );
    sandbox_name
}

fn generate_sha256(filename: &str) -> bool {
    let filepath = format!("{}.sha256", filename);
    let path = Path::new(filepath.as_str());
    if !path.exists() {
        return match sha256sum_of_file(filename) {
            Some(sum) => match fs::write(filepath, format!("{} generated.img\n", sum)) {
                Ok(_) => true,
                Err(s) => {
                    eprintln!("not ok {}", s);
                    false
                }
            },
            None => false,
        };
    }
    true
}

fn create_base_image_tmp_directory(directory: &str) -> io::Result<()> {
    fs::create_dir(format!(
        "./{}/{}/{}",
        BUILD_DIRECTORY, BASEIMAGE_DIRECTORY, directory
    ))
}

fn unpack_image(image_path: &str, directory: &str) -> bool {
    let complete_output = Command::new("unzip")
        .arg(image_path)
        .arg("-d")
        .arg(format!(
            "./{}/{}/{}",
            BUILD_DIRECTORY, BASEIMAGE_DIRECTORY, directory
        ))
        .output()
        .expect("failed to unpack image");
    complete_output.status.success()
}

fn read_configuration_of_base_image(directory: &str) -> Option<Configuration> {
    let content = fs::read_to_string(format!(
        "./{}/{}/{}/os-build/configuration.json",
        BUILD_DIRECTORY, BASEIMAGE_DIRECTORY, directory
    ))
    .expect("Something went wrong reading the configuration.json");
    if let Ok(configuration) = serde_json::from_str::<Configuration>(content.as_str()) {
        return Some(configuration);
    }
    None
}

fn move_prebuilt_image_to_sandbox(imagefile: &Imagefile) -> bool {
    let image = String::from(imagefile.preamble.get_filename());
    let imagepath = format!("./{}/generated.img", BUILD_DIRECTORY);
    let mut child = Command::new("cp")
        .arg(image)
        .arg(&imagepath)
        .spawn()
        .unwrap();
    child.wait().is_ok()
}

fn move_files_to_sandbox(imagefile: &mut Imagefile) -> bool {
    for (source, _) in imagefile.provisioners.clone().into_iter().filter_map(|p| {
        return match p.get_type() {
            FILE => {
                let cmd = p.get_command();
                let source = cmd.get(0).unwrap().to_string();
                let destination = cmd.get(1).unwrap().to_string();
                Some((source, destination))
            }
            SHELL => None,
        };
    }) {
        let path = &format!("./{}/{}", BUILD_DIRECTORY, source);
        let mut directory = path.split('/').collect::<Vec<&str>>();
        directory.remove(directory.len() - 1);
        let p = directory.join("/");
        fs::create_dir_all(p).unwrap();
        let mut child = Command::new("cp").arg(&source).arg(path).spawn().unwrap();

        print_message(
            &format!("moving {} to {}", source, path),
            child.wait().is_ok(),
        );
    }
    true
}

fn write_configuration(imagefile: &mut Imagefile) -> bool {
    let mut file = File::create(format!("{}/configuration.json", BUILD_DIRECTORY)).unwrap();
    imagefile.configuration.architecture = imagefile.architecture.clone();
    let bootconfig = group(&imagefile.configuration.bootconfigs);
    imagefile.configuration.bootconfigs.clear();
    imagefile.configuration.bootconfigs.push(bootconfig);
    file.write_all(
        serde_json::to_string(&imagefile.configuration)
            .unwrap()
            .as_bytes(),
    )
    .is_ok()
}

fn move_preseed_file(imagefile: &Imagefile) -> bool {
    let source_path = imagefile.preamble.get_preseed_file();
    if !source_path.is_empty() {
        let directory = source_path.split('/').collect::<Vec<&str>>();
        let path = &format!("./{}/http/{}", BUILD_DIRECTORY, directory.last().unwrap());
        fs::create_dir_all(&format!("./{}/http", BUILD_DIRECTORY)).unwrap();
        let child = Command::new("cp").arg(source_path).arg(path).spawn();
        return child.is_ok();
    }
    true
}
