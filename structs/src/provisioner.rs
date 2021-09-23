use crate::provisioner::Types::{FILE, SHELL};
use crate::utils;
use string_builder::Builder;

#[derive(Debug, Eq, PartialEq, Clone)]
pub enum Types {
    SHELL,
    FILE,
}

#[derive(Debug, Clone)]
pub struct Provisioner {
    provisioner: Types,
    command: Vec<String>,
}

impl Provisioner {
    /// # Errors
    ///
    /// Will return `Err` if `command` could not be parsed
    pub fn parse(provisioner: &Types, command: &str) -> Result<Provisioner, &'static str> {
        match provisioner {
            SHELL => parse_run(command),
            FILE => parse_file(command),
        }
    }

    #[must_use]
    pub fn to_pkr_hcl(&self) -> String {
        match self.provisioner {
            Types::SHELL => self.get_run(),
            Types::FILE => self.get_file(),
        }
    }

    fn get_run(&self) -> String {
        let mut builder = Builder::default();
        utils::ident_and_append(&mut builder, "provisioner \"shell\" {\n", 2);
        utils::add_indented_aligned_key_value(
            &mut builder,
            4,
            7,
            "inline",
            utils::vec_to_string(&self.command, true).as_str(),
        );
        utils::ident_and_append(&mut builder, "}\n", 2);
        builder.string().unwrap()
    }

    fn get_file(&self) -> String {
        let mut builder = Builder::default();
        utils::ident_and_append(&mut builder, "provisioner \"file\" {\n", 2);
        let source = self.command[0].clone();
        let destination = self.command[1].clone();
        utils::add_indented_aligned_key_value(
            &mut builder,
            4,
            11,
            "destination",
            utils::quote(&destination).as_str(),
        );
        utils::add_indented_aligned_key_value(
            &mut builder,
            4,
            11,
            "source",
            utils::quote(&source).as_str(),
        );
        utils::ident_and_append(&mut builder, "}\n", 2);
        builder.string().unwrap()
    }

    #[must_use]
    pub fn get_type(&self) -> Types {
        self.provisioner.clone()
    }

    #[must_use]
    pub fn get_command(&self) -> Vec<String> {
        self.command.clone()
    }
}

fn parse_run(command: &str) -> Result<Provisioner, &'static str> {
    if command.is_empty() {
        Err("Could not parse Run provisioner")
    } else {
        let mut vec = Vec::new();
        vec.push(command.to_string());
        Ok(Provisioner {
            provisioner: SHELL,
            command: vec,
        })
    }
}

fn parse_file(command: &str) -> Result<Provisioner, &'static str> {
    let args: Vec<&str> = command.split_whitespace().collect::<Vec<&str>>();
    if args.len() == 2 {
        let mut vec = Vec::new();
        vec.push(args[0].to_string()); // source
        vec.push(args[1].to_string()); // destination
        return Ok(Provisioner {
            provisioner: FILE,
            command: vec,
        });
    }
    Err("Could not parse File provisioner")
}

#[must_use]
pub fn group(provisioners: &[Provisioner]) -> Vec<Provisioner> {
    let mut vec = Vec::<Provisioner>::new();
    let mut candidates = Provisioner {
        provisioner: SHELL,
        command: Vec::new(),
    };
    for provisioner in provisioners.iter() {
        if provisioner.provisioner == SHELL {
            candidates.command.append(&mut provisioner.command.to_vec())
        }
        if provisioner.provisioner == FILE {
            if !candidates.command.is_empty() {
                vec.push(candidates.clone());
                candidates = Provisioner {
                    provisioner: SHELL,
                    command: Vec::new(),
                };
            }
            vec.push(provisioner.clone());
        }
    }
    if !candidates.command.is_empty() {
        vec.push(candidates);
    }
    vec
}
