use crate::config::get_power_commands_of;
use structs::node::Node;
use structs::power_action::Type;
use structs::power_action::Type::{OFF, ON, REBOOT};
use structs::power_action_set::PowerActionSet;

fn execute(powerset: PowerActionSet, action: &Type) -> bool {
    match powerset.get(action) {
        Ok(power_action) => power_action.execute(),
        Err(str) => {
            eprintln!("{}", str);
            false
        }
    }
}

#[allow(dead_code)]
pub fn power_off(node: &Node) -> bool {
    execute(get_power_commands_of(node), &OFF)
}

#[allow(dead_code)]
pub fn power_on(node: &Node) -> bool {
    execute(get_power_commands_of(node), &ON)
}

pub fn reboot(node: &Node) -> bool {
    execute(get_power_commands_of(node), &REBOOT)
}
