use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};

#[derive(Debug, Copy, Clone, PartialEq, Serialize, Deserialize)]
pub enum DriverAdapter {
    #[serde(rename = "planetscale")]
    PlanetScale,

    #[serde(rename = "neon:ws")]
    Neon,

    #[serde(rename = "pg")]
    Pg,

    #[serde(rename = "libsql")]
    LibSQL,

    #[serde(rename = "d1")]
    D1,

    #[serde(rename = "better-sqlite3")]
    BetterSQLite3,

    #[serde(rename = "mssql")]
    Mssql,

    #[serde(rename = "mariadb")]
    MariaDb,
}

impl From<String> for DriverAdapter {
    fn from(s: String) -> Self {
        let s = s.as_str();
        serde_json::from_str(s).unwrap_or_else(|_| panic!("Unknown driver adapter: {}", &s))
    }
}

impl From<DriverAdapter> for String {
    fn from(driver_adapter: DriverAdapter) -> String {
        serde_json::value::to_value(driver_adapter)
            .ok()
            .and_then(|v| v.as_str().map(|v| v.to_owned()))
            .unwrap()
    }
}

impl Display for DriverAdapter {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let s: String = (*self).into();
        write!(f, "{s}")
    }
}
