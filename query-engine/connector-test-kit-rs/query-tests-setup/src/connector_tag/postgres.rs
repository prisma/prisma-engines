use super::*;
use crate::{datamodel_rendering::SqlDatamodelRenderer, BoxFuture, TestError};
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
        "postgres"
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
    V9,
    V10,
    V11,
    V12,
    V13,
    V14,
    V15,
    PgBouncer,
    NeonJs,
    PgJs,
}

impl TryFrom<&str> for PostgresVersion {
    type Error = TestError;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        let version = match s {
            "9" => Self::V9,
            "10" => Self::V10,
            "11" => Self::V11,
            "12" => Self::V12,
            "13" => Self::V13,
            "14" => Self::V14,
            "15" => Self::V15,
            "pgbouncer" => Self::PgBouncer,
            "neon.js" => Self::NeonJs,
            "pg.js" => Self::PgJs,
            _ => return Err(TestError::parse_error(format!("Unknown Postgres version `{s}`"))),
        };

        Ok(version)
    }
}

impl ToString for PostgresVersion {
    fn to_string(&self) -> String {
        match self {
            PostgresVersion::V9 => "9",
            PostgresVersion::V10 => "10",
            PostgresVersion::V11 => "11",
            PostgresVersion::V12 => "12",
            PostgresVersion::V13 => "13",
            PostgresVersion::V14 => "14",
            PostgresVersion::V15 => "15",
            PostgresVersion::PgBouncer => "pgbouncer",
            PostgresVersion::NeonJs => "neon.js",
            PostgresVersion::PgJs => "pg.js",
        }
        .to_owned()
    }
}
