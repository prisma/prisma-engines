use std::fmt::{Display, Formatter};

use psl::builtin_connectors::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Copy, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) enum Provider {
    #[serde(rename = "postgres")]
    Postgres,

    #[serde(rename = "mysql")]
    Mysql,

    #[serde(rename = "sqlite")]
    Sqlite,

    #[serde(rename = "sqlserver")]
    SqlServer,

    #[serde(rename = "mongo")]
    Mongo,

    #[serde(rename = "cockroach")]
    Cockroach,
}

impl TryFrom<&str> for Provider {
    type Error = String;

    fn try_from(provider: &str) -> Result<Self, Self::Error> {
        if POSTGRES.is_provider(provider) {
            Ok(Provider::Postgres)
        } else if MYSQL.is_provider(provider) {
            Ok(Provider::Mysql)
        } else if SQLITE.is_provider(provider) {
            Ok(Provider::Sqlite)
        } else if MSSQL.is_provider(provider) {
            Ok(Provider::SqlServer)
        } else if MONGODB.is_provider(provider) {
            Ok(Provider::Mongo)
        } else if COCKROACH.is_provider(provider) {
            Ok(Provider::Cockroach)
        } else {
            Err(format!("Connector {} is not supported yet", provider))
        }
    }
}

impl From<Provider> for String {
    fn from(val: Provider) -> Self {
        serde_json::to_string(&val).unwrap()
    }
}

impl Display for Provider {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let s: String = (*self).into();
        write!(f, "{}", s)
    }
}
