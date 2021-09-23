use itertools::Itertools;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct BootConfig {
    files: Vec<String>,
}

impl BootConfig {
    #[must_use]
    pub fn new() -> Self {
        BootConfig { files: Vec::new() }
    }

    /// # Errors
    ///
    /// Will return `Err` if `line` could not be parsed
    pub fn parse(line: &str) -> Result<BootConfig, &'static str> {
        let files = line
            .split_whitespace()
            .map(std::string::ToString::to_string)
            .collect::<Vec<String>>();
        if !files.is_empty() {
            return Ok(BootConfig { files });
        }
        Err("Could not parse BootConfig")
    }

    #[must_use]
    pub fn get_files(&self) -> Vec<String> {
        self.files.clone()
    }
}

impl Default for BootConfig {
    fn default() -> Self {
        Self::new()
    }
}

#[must_use]
pub fn group(bootconfigs: &[BootConfig]) -> BootConfig {
    let mut bootconfig = BootConfig { files: Vec::new() };
    bootconfigs
        .iter()
        .for_each(|b| bootconfig.files.append(&mut b.files.clone()));
    bootconfig.files = bootconfig.files.into_iter().unique().collect();
    bootconfig
}
