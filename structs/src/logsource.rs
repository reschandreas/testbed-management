use serde::{Deserialize, Serialize};

#[derive(Debug, Eq, PartialEq, Serialize, Deserialize, Clone)]
pub enum LogSourceTypes {
    HOST,
    SERIAL,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LogSource {
    pub path: String,
    pub source: LogSourceTypes,
}

impl LogSource {
    #[must_use]
    pub fn new(path: String, source: LogSourceTypes) -> Self {
        LogSource { path, source }
    }

    #[must_use]
    pub fn host(path: String) -> Self {
        LogSource::new(path, LogSourceTypes::HOST)
    }

    #[must_use]
    pub fn serial(path: String) -> Self {
        LogSource::new(path, LogSourceTypes::SERIAL)
    }
}
