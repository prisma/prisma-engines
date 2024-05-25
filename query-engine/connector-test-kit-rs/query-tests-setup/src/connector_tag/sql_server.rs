use std::fmt::Display;

use quaint::{prelude::Queryable, single::Quaint};

use super::*;
use crate::{datamodel_rendering::SqlDatamodelRenderer, BoxFuture, TestError};

#[derive(Debug, Default, Clone)]
pub(crate) struct SqlServerConnectorTag;

impl ConnectorTagInterface for SqlServerConnectorTag {
    fn raw_execute<'a>(&'a self, query: &'a str, connection_url: &'a str) -> BoxFuture<'a, Result<(), TestError>> {
        Box::pin(async move {
            let conn = Quaint::new(connection_url).await?;
            Ok(conn.raw_cmd(query).await?)
        })
    }

    fn datamodel_provider(&self) -> &'static str {
        "sqlserver"
    }

    fn datamodel_renderer(&self) -> Box<dyn DatamodelRenderer> {
        Box::new(SqlDatamodelRenderer::new())
    }

    fn capabilities(&self) -> ConnectorCapabilities {
        psl::builtin_connectors::MSSQL.capabilities()
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SqlServerVersion {
    V2017,
    V2019,
    V2022,
}

impl TryFrom<&str> for SqlServerVersion {
    type Error = TestError;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        let version = match s {
            "2017" => Self::V2017,
            "2019" => Self::V2019,
            "2022" => Self::V2022,
            _ => return Err(TestError::parse_error(format!("Unknown SqlServer version `{s}`"))),
        };

        Ok(version)
    }
}

impl Display for SqlServerVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SqlServerVersion::V2017 => f.write_str("2017"),
            SqlServerVersion::V2019 => f.write_str("2019"),
            SqlServerVersion::V2022 => f.write_str("2022"),
        }
    }
}
