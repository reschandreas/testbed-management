use crate::deployment::Deployment;
use crate::utils::get_cell_content_of_date;
use chrono::NaiveDateTime;
use prettytable::Cell;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct DeploymentRow {
    pub id: Option<i64>,
    pub name: String,
    pub start: Option<NaiveDateTime>,
    pub end: Option<NaiveDateTime>,
    pub owner: String,
    pub services: Option<usize>,
}

impl DeploymentRow {
    #[must_use]
    pub fn new(deployment: Deployment, services: Option<usize>) -> Self {
        DeploymentRow {
            id: deployment.id,
            name: deployment.name,
            start: Some(deployment.start),
            end: deployment.end,
            owner: deployment.owner,
            services,
        }
    }

    #[must_use]
    pub fn get_cells(&self) -> Vec<Cell> {
        let mut cells: Vec<prettytable::Cell> = Vec::new();
        cells.push(Cell::new(&self.id.unwrap().to_string()));
        cells.push(Cell::new(&self.name));
        cells.push(get_cell_content_of_date(&self.start));
        cells.push(get_cell_content_of_date(&self.end));
        cells.push(Cell::new(&self.owner));
        match self.services {
            Some(number) => cells.push(Cell::new(&number.to_string())),
            None => cells.push(Cell::new("\u{2014}")),
        }
        cells
    }
}
