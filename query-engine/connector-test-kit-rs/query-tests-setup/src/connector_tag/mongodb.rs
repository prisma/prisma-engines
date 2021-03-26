use super::*;
use crate::{MongoDbSchemaRenderer, TestError, TestResult};

#[derive(Debug, Default, Clone, Copy)]
pub struct MongoDbConnectorTag {
    version: Option<MongoDbVersion>,
}

impl ConnectorTagInterface for MongoDbConnectorTag {
    fn datamodel_provider(&self) -> &'static str {
        "mongodb"
    }

    fn datamodel_renderer(&self) -> Box<dyn DatamodelRenderer> {
        Box::new(MongoDbSchemaRenderer::new())
    }

    fn connection_string(&self, database: &str, is_ci: bool) -> String {
        match self.version {
            Some(MongoDbVersion::V4) if is_ci => format!(
                "mongodb://prisma:prisma@test-db-mongodb-4:27017/{}?authSource=admin",
                database
            ),
            Some(MongoDbVersion::V4) => {
                format!("mongodb://prisma:prisma@127.0.0.1:27017/{}?authSource=admin", database)
            }

            None => unreachable!("A versioned connector must have a concrete version to run."),
        }
        .to_string()
    }

    fn capabilities(&self) -> Vec<ConnectorCapability> {
        todo!()
    }

    fn as_parse_pair(&self) -> (String, Option<String>) {
        let version = self.version.as_ref().map(ToString::to_string);
        ("mongodb".to_owned(), version)
    }

    fn is_versioned(&self) -> bool {
        true
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MongoDbVersion {
    V4,
}

impl MongoDbConnectorTag {
    pub fn new(version: Option<&str>) -> TestResult<Self> {
        let version = match version {
            Some(v) => Some(MongoDbVersion::try_from(v)?),
            None => None,
        };

        Ok(Self { version })
    }

    /// Returns all versions of this connector.
    pub fn all() -> Vec<Self> {
        vec![Self {
            version: Some(MongoDbVersion::V4),
        }]
    }
}

impl PartialEq for MongoDbConnectorTag {
    fn eq(&self, other: &Self) -> bool {
        match (self.version, other.version) {
            (None, None) | (Some(_), None) | (None, Some(_)) => true,
            (Some(v1), Some(v2)) => v1 == v2,
        }
    }
}

impl TryFrom<&str> for MongoDbVersion {
    type Error = TestError;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        let version = match s {
            "4" => Self::V4,
            _ => return Err(TestError::parse_error(format!("Unknown MongoDB version `{}`", s))),
        };

        Ok(version)
    }
}

impl ToString for MongoDbVersion {
    fn to_string(&self) -> String {
        match self {
            MongoDbVersion::V4 => "4",
        }
        .to_owned()
    }
}
