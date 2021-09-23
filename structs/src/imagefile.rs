use crate::architecture::Architecture;
use crate::bootconfig::BootConfig;
use crate::configuration::Configuration;
use crate::partition::Partition;
use crate::post_provisioner::PostProvisioner;
use crate::post_provisioner::Types::LocalShell;
use crate::preamble::Preamble;
use crate::provisioner;
use crate::provisioner::Provisioner;
use crate::utils;
use std::collections::HashMap;
use string_builder::Builder;

pub struct Imagefile {
    pub name: String,
    pub architecture: Architecture,
    pub preamble: Box<dyn Preamble>,
    pub partitions: HashMap<String, Partition>,
    pub configuration: Configuration,
    pub provisioners: Vec<Provisioner>,
    pub post_provisioners: Vec<PostProvisioner>,
}

impl Imagefile {
    #[must_use]
    pub fn new(name: String, architecture: &Architecture) -> Self {
        Imagefile {
            name,
            architecture: architecture.clone(),
            preamble: architecture.get_preamble(),
            partitions: HashMap::new(),
            configuration: Configuration::default(),
            provisioners: Vec::new(),
            post_provisioners: match architecture {
                Architecture::ARM64 | Architecture::ARM32 => Vec::new(),
                Architecture::X86 => {
                    let mut vec = Vec::new();
                    vec.push(PostProvisioner {
                        provisioner: LocalShell,
                        command: {
                            let mut v = Vec::new();
                            v.push(
                                "mv output/${var.vmname}-disk001.vmdk generated.vmdk".to_string(),
                            );
                            v.push("mv output/${var.vmname}.ovf generated.ovf".to_string());
                            v.push("rm -rf output/".to_string());
                            v
                        },
                    });
                    vec
                }
            },
        }
    }

    pub fn as_pkr_hcl(&mut self) -> String {
        let mut builder = Builder::default();
        builder.append(self.preamble.to_pkr_hcl());
        self.configuration.partitions_to_vec(&self.partitions);
        for partition in self.partitions.values() {
            if self
                .configuration
                .partitions
                .iter()
                .all(|p| !p.get_mountpoint().eq(&partition.get_mountpoint()))
            {
                self.configuration.partitions.push(partition.clone())
            }
        }
        if !self.configuration.partitions.is_empty() {
            match &self.architecture {
                Architecture::ARM32 | Architecture::ARM64 => {
                    for partition in &self.configuration.partitions {
                        builder.append(partition.to_pkr_hcl());
                    }
                }
                Architecture::X86 => {}
            }
        }
        builder.append("\n}\n");
        builder.append("build {\n");
        utils::ident_and_append(
            &mut builder,
            &format!(
                "sources = [\"source.{}.imagefile\"]\n\n",
                self.preamble.get_packer_plugin()
            ),
            2,
        );
        for provisioner in provisioner::group(&self.provisioners) {
            builder.append(provisioner.to_pkr_hcl());
            builder.append("\n");
        }
        for provisioner in &self.post_provisioners {
            builder.append(provisioner.to_pkr_hcl());
            builder.append("\n");
        }
        builder.append("\n}\n");
        builder.string().unwrap()
    }

    #[must_use]
    pub fn get_boot_files(&self) -> BootConfig {
        crate::bootconfig::group(&self.configuration.bootconfigs)
    }
}
