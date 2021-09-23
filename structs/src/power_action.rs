use std::process::Command;

#[derive(Debug, Eq, PartialEq, Clone)]
pub enum Type {
    ON,
    OFF,
    REBOOT,
}

#[derive(Debug, Clone)]
pub struct PowerAction {
    action: Type,
    command: String,
    arguments: Vec<String>,
}

impl PowerAction {
    /// # Errors
    ///
    /// Will return `Err` if `command` could not be parsed
    pub fn parse(action: Type, command: &str) -> Result<Self, String> {
        let mut strings = command
            .split_whitespace()
            .map(std::string::ToString::to_string)
            .collect::<Vec<String>>();
        if strings.is_empty() {
            return Err("Could not parse power command".to_string());
        }
        let application = strings[0].clone();
        strings.remove(0);
        let mut arguments = Vec::new();
        for string in strings {
            arguments.push(string);
        }
        Ok(PowerAction {
            action,
            command: application,
            arguments,
        })
    }

    #[must_use]
    pub fn get_action(&self) -> Type {
        self.action.clone()
    }

    #[must_use]
    pub fn get_command(&self) -> String {
        self.command.clone()
    }

    #[must_use]
    pub fn execute(self) -> bool {
        let child = Command::new(self.command)
            .args(self.arguments)
            .spawn()
            .unwrap();
        child.wait_with_output().unwrap().status.success()
    }
}
