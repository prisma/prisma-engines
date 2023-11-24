use super::*;
use crate::{BoxFuture, SqlDatamodelRenderer};
use quaint::{prelude::Queryable, single::Quaint};

#[derive(Debug, Default)]
pub struct SqliteConnectorTag;

impl ConnectorTagInterface for SqliteConnectorTag {
    fn raw_execute<'a>(&'a self, query: &'a str, connection_url: &'a str) -> BoxFuture<'a, Result<(), TestError>> {
        Box::pin(async move {
            let conn = Quaint::new(connection_url).await?;
            Ok(conn.raw_cmd(query).await?)
        })
    }

    fn datamodel_provider(&self) -> &'static str {
        "sqlite"
    }

    fn datamodel_renderer(&self) -> Box<dyn DatamodelRenderer> {
        Box::new(SqlDatamodelRenderer::new())
    }

    fn capabilities(&self) -> ConnectorCapabilities {
        psl::builtin_connectors::SQLITE.capabilities()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SqliteVersion {
    V3,
    LibsqlJS,
}

impl ToString for SqliteVersion {
    fn to_string(&self) -> String {
        match self {
            SqliteVersion::V3 => "3".to_string(),
            SqliteVersion::LibsqlJS => "libsql.js".to_string(),
        }
    }
}

impl TryFrom<&str> for SqliteVersion {
    type Error = TestError;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        let version = match s {
            "3" => Self::V3,
            "libsql.js" => Self::LibsqlJS,
            _ => return Err(TestError::parse_error(format!("Unknown SQLite version `{s}`"))),
        };
        Ok(version)
    }
}
