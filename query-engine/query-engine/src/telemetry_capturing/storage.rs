use super::models;
use super::settings::Settings;

#[derive(Debug, Default)]
pub struct Storage {
    pub traces: Vec<models::ExportedSpan>,
    pub logs: Vec<models::ExportedLog>,
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
