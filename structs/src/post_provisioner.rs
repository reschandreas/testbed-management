use crate::utils;
use string_builder::Builder;

#[derive(Debug, Eq, PartialEq, Clone)]
pub enum Types {
    LocalShell,
}

#[derive(Debug, Clone)]
pub struct PostProvisioner {
    pub(crate) provisioner: Types,
    pub(crate) command: Vec<String>,
}

impl PostProvisioner {
    #[must_use]
    pub fn new(provisioner: Types, command: &str) -> Self {
        let mut vec = Vec::new();
        vec.push(command.to_string());

        Self {
            provisioner,
            command: vec,
        }
    }

    #[must_use]
    pub fn to_pkr_hcl(&self) -> String {
        match self.provisioner {
            Types::LocalShell => self.get_localshell(),
        }
    }

    fn get_localshell(&self) -> String {
        let mut builder = Builder::default();
        utils::ident_and_append(&mut builder, "post-processor \"shell-local\" {\n", 2);
        utils::add_indented_aligned_key_value(
            &mut builder,
            4,
            7,
            "inline",
            &utils::vec_to_string(&self.command, true),
        );
        utils::ident_and_append(&mut builder, "}\n", 2);
        builder.string().unwrap()
    }
}
