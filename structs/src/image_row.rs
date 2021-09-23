use crate::configuration::Configuration;
use prettytable::Cell;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct ImageRow {
    pub filename: String,
    pub configuration: Option<Configuration>,
}

impl ImageRow {
    #[must_use]
    pub fn new(filename: String, configuration: Option<Configuration>) -> Self {
        ImageRow {
            filename,
            configuration,
        }
    }

    #[must_use]
    pub fn get_cells(&self) -> Vec<Cell> {
        let mut cells: Vec<prettytable::Cell> = Vec::new();
        cells.push(Cell::new(&self.filename));
        match &self.configuration {
            Some(config) => {
                cells.push(Cell::new(config.architecture.get_name()));
                cells.push(Cell::new(if config.on_device { "yes" } else { "no" }));
            }
            None => {
                cells.push(Cell::new("\u{2014}"));
                cells.push(Cell::new("\u{2014}"));
            }
        }
        cells
    }
}
