use crate::node::Node;
use crate::utils::get_cell_content_of_string;
use prettytable::Cell;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct NodeRow {
    pub id: String,
    pub name: String,
    pub mac_address: String,
    pub tftp_prefix: String,
    pub serial_number: String,
    pub status: Option<bool>,
    pub hostname: Option<String>,
    pub ipv4_address: Option<String>,
    pub usable: bool,
}

impl NodeRow {
    #[must_use]
    pub fn new(
        node: Node,
        status: Option<bool>,
        hostname: Option<String>,
        ipv4_address: Option<String>,
        usable: bool,
    ) -> Self {
        NodeRow {
            id: node.id,
            name: node.name,
            mac_address: node.mac_address,
            tftp_prefix: node.tftp_prefix,
            serial_number: node.serial_number,
            status,
            hostname,
            ipv4_address,
            usable,
        }
    }

    #[must_use]
    pub fn get_cells(&self) -> Vec<Cell> {
        let mut cells: Vec<prettytable::Cell> = Vec::new();
        cells.push(get_cell_content_of_string(&self.id));
        cells.push(get_cell_content_of_string(&self.name));
        cells.push(get_cell_content_of_string(&self.mac_address));
        cells.push(get_cell_content_of_string(&self.tftp_prefix));
        cells.push(get_cell_content_of_string(&self.serial_number));
        if let Some(is_up) = self.status {
            cells.push(Cell::new(if is_up { "up" } else { "down" }));
        } else {
            cells.push(Cell::new("\u{2014}"));
        }
        for value in [&self.hostname, &self.ipv4_address].to_vec() {
            cells.push(get_cell_content_of_string(
                value.as_ref().unwrap_or(&String::from("\u{2014}")),
            ));
        }
        cells.push(Cell::new(if self.usable { "yes" } else { "no" }));
        cells
    }
}
