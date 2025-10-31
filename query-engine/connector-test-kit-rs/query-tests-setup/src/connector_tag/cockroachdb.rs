use super::*;
use crate::{BoxFuture, datamodel_rendering::SqlDatamodelRenderer};
use psl::datamodel_connector::ConnectorCapabilities;
use quaint::{prelude::Queryable, single::Quaint};

#[derive(Debug, Default, Clone)]
pub(crate) struct CockroachDbConnectorTag;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CockroachDbVersion {
    V243,
    V251,
    V252,
    PgJsWasm,
}

impl TryFrom<&str> for CockroachDbVersion {
    type Error = TestError;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        let version = match s {
            "24.3" => Self::V243,
            "25.1" => Self::V251,
            "25.2" => Self::V252,
            "pg.js.wasm" => Self::PgJsWasm,
            _ => return Err(TestError::parse_error(format!("Unknown CockroachDB version `{s}`"))),
        };

        Ok(version)
    }
}

impl fmt::Display for CockroachDbVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CockroachDbVersion::V243 => f.write_str("24.3"),
            CockroachDbVersion::V251 => f.write_str("25.1"),
            CockroachDbVersion::V252 => f.write_str("25.2"),
            CockroachDbVersion::PgJsWasm => f.write_str("pg.js.wasm"),
        }
    }
}

impl Default for CockroachDbVersion {
    fn default() -> Self {
        Self::V243
    }
}

impl ConnectorTagInterface for CockroachDbConnectorTag {
    fn raw_execute<'a>(&'a self, query: &'a str, connection_url: &'a str) -> BoxFuture<'a, Result<(), TestError>> {
        Box::pin(async move {
            let conn = Quaint::new(connection_url).await?;
            Ok(conn.raw_cmd(query).await?)
        })
    }

    fn datamodel_provider(&self) -> &'static str {
        "cockroachdb"
    }

    fn datamodel_renderer(&self) -> Box<dyn DatamodelRenderer> {
        Box::new(SqlDatamodelRenderer::new())
    }

    fn capabilities(&self) -> ConnectorCapabilities {
        psl::builtin_connectors::COCKROACH.capabilities()
    }
}
