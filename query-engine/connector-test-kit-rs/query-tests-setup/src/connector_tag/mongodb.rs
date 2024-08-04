use std::fmt::Display;

use super::*;
use crate::{MongoDbSchemaRenderer, TestError};
use psl::builtin_connectors::MONGODB;

#[derive(Debug, Default, Clone)]
pub(crate) struct MongoDbConnectorTag;

impl ConnectorTagInterface for MongoDbConnectorTag {
    fn raw_execute(&self, _query: &str, _connection_url: &str) -> BoxFuture<Result<(), TestError>> {
        panic!("raw_execute is not supported for MongoDB yet");
    }

    fn datamodel_provider(&self) -> &'static str {
        "mongodb"
    }

    fn datamodel_renderer(&self) -> Box<dyn DatamodelRenderer> {
        Box::new(MongoDbSchemaRenderer::new())
    }

    fn capabilities(&self) -> ConnectorCapabilities {
        MONGODB.capabilities()
    }

    fn relation_mode(&self) -> &'static str {
        "prisma"
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MongoDbVersion {
    V4_2,
    V4_4,
    V5,
}

impl TryFrom<&str> for MongoDbVersion {
    type Error = TestError;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        let version = match s {
            "4.4" => Self::V4_4,
            "4.2" => Self::V4_2,
            "5" => Self::V5,
            _ => return Err(TestError::parse_error(format!("Unknown MongoDB version `{s}`"))),
        };

        Ok(version)
    }
}

impl Display for MongoDbVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MongoDbVersion::V4_4 => f.write_str("4.4"),
            &MongoDbVersion::V4_2 => f.write_str("4.2"),
            MongoDbVersion::V5 => f.write_str("5"),
        }
    }
}
