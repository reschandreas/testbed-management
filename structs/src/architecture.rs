use crate::arm_preamble::ArmPreamble;
use crate::preamble::Preamble;
use crate::x86_preamble::X86Preamble;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone, Eq)]
pub enum Architecture {
    ARM32,
    ARM64,
    X86,
}

impl Architecture {
    #[must_use]
    pub fn get_name(&self) -> &'static str {
        match self {
            Architecture::ARM32 => "ARM32",
            Architecture::ARM64 => "ARM64",
            Architecture::X86 => "X86",
        }
    }

    #[must_use]
    pub fn get_preamble(&self) -> Box<dyn Preamble> {
        match self {
            Architecture::ARM32 | Architecture::ARM64 => Box::new(ArmPreamble::default()),
            Architecture::X86 => Box::new(X86Preamble::default()),
        }
    }
    /// # Errors
    ///
    /// Will return `Err` if `line` could not be parsed
    pub fn parse(line: &str) -> Result<Architecture, &'static str> {
        match line {
            "ARM32" => Ok(Architecture::ARM32),
            "ARM64" => Ok(Architecture::ARM64),
            "X86" => Ok(Architecture::X86),
            _ => Err("not supported"),
        }
    }
}

impl PartialEq<Self> for Architecture {
    fn eq(&self, other: &Self) -> bool {
        self.get_name().eq(other.get_name())
    }
}
