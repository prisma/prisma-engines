use std::fmt::Display;

use super::*;
use crate::{BoxFuture, TestError, datamodel_rendering::SqlDatamodelRenderer};
use quaint::{prelude::Queryable, single::Quaint};

#[derive(Debug, Default, Clone)]
pub(crate) struct PostgresConnectorTag;

impl ConnectorTagInterface for PostgresConnectorTag {
    fn raw_execute<'a>(&'a self, query: &'a str, connection_url: &'a str) -> BoxFuture<'a, Result<(), TestError>> {
        Box::pin(async move {
            let conn = Quaint::new(connection_url).await?;
            Ok(conn.raw_cmd(query).await?)
        })
    }

    fn datamodel_provider(&self) -> &'static str {
        "postgresql"
    }

    fn datamodel_renderer(&self) -> Box<dyn DatamodelRenderer> {
        Box::new(SqlDatamodelRenderer::new())
    }

    fn capabilities(&self) -> ConnectorCapabilities {
        psl::builtin_connectors::POSTGRES.capabilities()
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PostgresVersion {
    V12,
    V13,
    V14,
    V15,
    V16,
    PgBouncer,
    NeonJsWasm,
    PgJsWasm,
}

impl TryFrom<&str> for PostgresVersion {
    type Error = TestError;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        let version = match s {
            "12" => Self::V12,
            "13" => Self::V13,
            "14" => Self::V14,
            "15" => Self::V15,
            "16" => Self::V16,
            "pgbouncer" => Self::PgBouncer,
            "pg.js.wasm" => Self::PgJsWasm,
            "neon.js.wasm" => Self::NeonJsWasm,
            _ => return Err(TestError::parse_error(format!("Unknown Postgres version `{s}`"))),
        };

        Ok(version)
    }
}

impl Display for PostgresVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PostgresVersion::V12 => f.write_str("12"),
            PostgresVersion::V13 => f.write_str("13"),
            PostgresVersion::V14 => f.write_str("14"),
            PostgresVersion::V15 => f.write_str("15"),
            PostgresVersion::V16 => f.write_str("16"),
            PostgresVersion::PgBouncer => f.write_str("pgbouncer"),
            PostgresVersion::PgJsWasm => f.write_str("pg.js.wasm"),
            PostgresVersion::NeonJsWasm => f.write_str("pg.js.wasm"),
        }
    }
}
