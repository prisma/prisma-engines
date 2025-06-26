use super::*;
use crate::{datamodel_rendering::SqlDatamodelRenderer, BoxFuture};
use psl::datamodel_connector::ConnectorCapabilities;
use quaint::{prelude::Queryable, single::Quaint};

#[derive(Debug, Default, Clone)]
pub(crate) struct CockroachDbConnectorTag;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CockroachDbVersion {
    V231,
    V222,
    V221,
    PgJsWasm,
}

impl TryFrom<&str> for CockroachDbVersion {
    type Error = TestError;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        let version = match s {
            "22.1" => Self::V221,
            "22.2" => Self::V222,
            "23.1" => Self::V231,
            "pg.js.wasm" => Self::PgJsWasm,
            _ => return Err(TestError::parse_error(format!("Unknown CockroachDB version `{s}`"))),
        };

        Ok(version)
    }
}

impl fmt::Display for CockroachDbVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CockroachDbVersion::V231 => f.write_str("23.1"),
            CockroachDbVersion::V222 => f.write_str("22.2"),
            CockroachDbVersion::V221 => f.write_str("22.1"),
            CockroachDbVersion::PgJsWasm => f.write_str("pg.js.wasm"),
        }
    }
}

impl Default for CockroachDbVersion {
    fn default() -> Self {
        Self::V221
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
