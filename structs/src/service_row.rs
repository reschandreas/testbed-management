use crate::service::Service;
use crate::utils::{get_cell_content_of_date, get_cell_content_of_option};
use chrono::NaiveDateTime;
use prettytable::Cell;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct ServiceRow {
    pub id: i64,
    pub name: String,
    pub image: String,
    pub node: String,
    pub deployment: String,
    pub hostname: String,
    pub ipv4_address: Option<String>,
    pub start: Option<NaiveDateTime>,
    pub end: Option<NaiveDateTime>,
    pub replicas: i64,
}

impl ServiceRow {
    #[must_use]
    pub fn new(service: &Service, node: String, deployment: String) -> Self {
        ServiceRow {
            id: service.id.unwrap(),
            name: service.name.clone(),
            image: service.image.clone(),
            node,
            deployment,
            hostname: service.hostname.clone(),
            ipv4_address: service.ipv4_address.clone(),
            start: Some(service.start),
            end: service.end,
            replicas: service.replicas,
        }
    }

    #[must_use]
    pub fn get_cells(&self) -> Vec<Cell> {
        let mut cells: Vec<prettytable::Cell> = Vec::new();
        cells.push(Cell::new(&self.id.to_string()));
        cells.push(Cell::new(&self.name));
        cells.push(Cell::new(&self.image));
        cells.push(Cell::new(&self.node));
        cells.push(Cell::new(&self.deployment));
        cells.push(Cell::new(&self.hostname));
        cells.push(get_cell_content_of_option(&self.ipv4_address));
        cells.push(get_cell_content_of_date(&self.start));
        cells.push(get_cell_content_of_date(&self.end));
        cells.push(Cell::new(&self.replicas.to_string()));
        cells
    }
}
