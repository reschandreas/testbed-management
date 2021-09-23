use crate::power_action::{PowerAction, Type};

#[derive(Debug, Clone)]
pub struct PowerActionSet {
    on: Result<PowerAction, String>,
    off: Result<PowerAction, String>,
    reboot: Result<PowerAction, String>,
}

impl PowerActionSet {
    #[must_use]
    pub fn new(
        on: Result<PowerAction, String>,
        off: Result<PowerAction, String>,
        reboot: Result<PowerAction, String>,
    ) -> Self {
        PowerActionSet { on, off, reboot }
    }

    /// # Errors
    ///
    /// Will return `Err` if `action` is returning a `Err`
    pub fn get(self, action: &Type) -> Result<PowerAction, String> {
        match action {
            crate::power_action::Type::ON => self.on,
            crate::power_action::Type::OFF => self.off,
            crate::power_action::Type::REBOOT => self.reboot,
        }
    }
}
