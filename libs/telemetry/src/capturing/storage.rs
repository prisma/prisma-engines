use super::settings::Settings;
use crate::models;

#[derive(Debug, Default)]
pub struct Storage {
    pub traces: Vec<models::TraceSpan>,
    pub logs: Vec<models::LogEvent>,
    pub settings: Settings,
}

impl From<Settings> for Storage {
    fn from(settings: Settings) -> Self {
        Self {
            traces: Default::default(),
            logs: Default::default(),
            settings,
        }
    }
}
