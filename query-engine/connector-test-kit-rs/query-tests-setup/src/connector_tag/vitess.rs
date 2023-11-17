use super::*;
use crate::{BoxFuture, SqlDatamodelRenderer};
use quaint::{prelude::Queryable, single::Quaint};
use std::{fmt::Display, str::FromStr};

#[derive(Debug, Default, Clone)]
pub(crate) struct VitessConnectorTag;

impl ConnectorTagInterface for VitessConnectorTag {
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

    fn relation_mode(&self) -> &'static str {
        "prisma"
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum VitessVersion {
    V8_0,
}

impl FromStr for VitessVersion {
    type Err = TestError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let version = match s {
            "8.0" => Self::V8_0,
            _ => return Err(TestError::parse_error(format!("Unknown Vitess version `{s}`"))),
        };

        Ok(version)
    }
}

impl Display for VitessVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::V8_0 => write!(f, "8.0"),
        }
    }
}
