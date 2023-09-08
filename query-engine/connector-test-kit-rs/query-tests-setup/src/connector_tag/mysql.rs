use super::*;
use crate::{datamodel_rendering::SqlDatamodelRenderer, BoxFuture, TestError};
use quaint::{prelude::Queryable, single::Quaint};

#[derive(Debug, Default, Clone)]
pub(crate) struct MySqlConnectorTag;

impl ConnectorTagInterface for MySqlConnectorTag {
    fn raw_execute<'a>(&'a self, query: &'a str, connection_url: &'a str) -> BoxFuture<'a, Result<(), TestError>> {
        Box::pin(async move {
            let conn = Quaint::new(connection_url).await?;
            Ok(conn.raw_cmd(query).await?)
        })
    }

    fn datamodel_provider(&self) -> &'static str {
        "mysql"
    }

    fn datamodel_renderer(&self) -> Box<dyn DatamodelRenderer> {
        Box::new(SqlDatamodelRenderer::new())
    }

    fn capabilities(&self) -> ConnectorCapabilities {
        psl::builtin_connectors::MYSQL.capabilities()
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MySqlVersion {
    V5_6,
    V5_7,
    V8,
    MariaDb,
}

impl TryFrom<&str> for MySqlVersion {
    type Error = TestError;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        let version = match s {
            "5.6" => Self::V5_6,
            "5.7" => Self::V5_7,
            "8" => Self::V8,
            "mariadb" => Self::MariaDb,
            _ => return Err(TestError::parse_error(format!("Unknown MySQL version `{s}`"))),
        };

        Ok(version)
    }
}

impl ToString for MySqlVersion {
    fn to_string(&self) -> String {
        match self {
            MySqlVersion::V5_6 => "5.6",
            MySqlVersion::V5_7 => "5.7",
            MySqlVersion::V8 => "8",
            MySqlVersion::MariaDb => "mariadb",
        }
        .to_owned()
    }
}
