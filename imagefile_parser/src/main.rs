mod builder;
mod client;
mod parser;

extern crate clap;
extern crate structs;
use crate::client::push_image;
use clap::{App, Arg};
use colored::Colorize;
use structs::utils::print_message;

pub const OS_IMAGES_DIR: &str = "/etc/cluster-manager/os_images";

fn main() {
    let matches = App::new("imagefile-parser")
        .version("0.1")
        .author("Andreas Resch <andreas@resch.io>")
        .about("Takes a custom Imagefile and transforms it to a valid pkr.hcl")
        .arg(
            Arg::with_name("input")
                .short("i")
                .long("input")
                .value_name("FILE")
                .help("Sets the input file")
                .required(true)
                .takes_value(true),
        )
        .arg(
            Arg::with_name("output")
                .short("o")
                .long("output")
                .value_name("FILE")
                .help("Sets the output file, defaults to image.pkr.hcl")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("build")
                .long("build")
                .help("Build image")
                .takes_value(false),
        )
        .arg(
            Arg::with_name("tag")
                .long("tag")
                .help("Tag image")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("push")
                .long("push")
                .help("Push image")
                .takes_value(false),
        )
        .get_matches();

    if let Some(input) = matches.value_of("input") {
        let output_name = matches
            .value_of("output")
            .or(Some("image.pkr.hcl"))
            .unwrap();
        let imagefile = self::parser::parse(input);
        print_message("parsing input file", imagefile.is_some());
        if let Some(mut image) = imagefile {
            if matches.is_present("build") {
                let random_tag = structs::utils::get_random_name();
                let tag = matches
                    .value_of("tag")
                    .or_else(|| Some(random_tag.as_str()))
                    .unwrap();
                let status = self::builder::build(&mut image, output_name, tag);
                if matches.is_present("push") && status {
                    print_message(
                        "push image to server",
                        push_image(tag, &format!("{}.zip", tag)).unwrap(),
                    );
                }
            }
        }
    } else {
        eprintln!("{}", "No input file found, exiting...".red());
    }
}
