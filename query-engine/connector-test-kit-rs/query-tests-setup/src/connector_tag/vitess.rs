use super::*;
use crate::{SqlDatamodelRenderer, TestResult};
use std::{fmt::Display, str::FromStr};

#[derive(Debug, Default, Clone)]
pub struct VitessConnectorTag {
    version: Option<VitessVersion>,
}

impl ConnectorTagInterface for VitessConnectorTag {
    fn datamodel_provider(&self) -> &'static str {
        "mysql"
    }

    fn datamodel_renderer(&self) -> Box<dyn DatamodelRenderer> {
        Box::new(SqlDatamodelRenderer::new())
    }

    fn connection_string(
        &self,
        _database: &str,
        _is_ci: bool,
        _is_multi_schema: bool,
        _: Option<&'static str>,
    ) -> String {
        match self.version {
            Some(VitessVersion::V5_7) => "mysql://root@localhost:33577/test".into(),
            Some(VitessVersion::V8_0) => "mysql://root@localhost:33807/test".into(),
            None => unreachable!("A versioned connector must have a concrete version to run."),
        }
    }

    fn capabilities(&self) -> ConnectorCapabilities {
        psl::builtin_connectors::MYSQL.capabilities()
    }

    fn as_parse_pair(&self) -> (String, Option<String>) {
        let version = self.version.as_ref().map(ToString::to_string);
        ("vitess".to_owned(), version)
    }

    fn is_versioned(&self) -> bool {
        true
    }

    fn relation_mode(&self) -> &'static str {
        "prisma"
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum VitessVersion {
    V5_7,
    V8_0,
}

impl VitessConnectorTag {
    pub fn new(version: Option<&str>) -> TestResult<Self> {
        let version = match version {
            Some(v) => Some(v.parse()?),
            None => None,
        };

        Ok(Self { version })
    }

    /// Returns all versions of this connector.
    pub fn all() -> Vec<Self> {
        vec![
            Self {
                version: Some(VitessVersion::V5_7),
            },
            Self {
                version: Some(VitessVersion::V8_0),
            },
        ]
    }

    /// Get a reference to the vitess connector tag's version.
    pub fn version(&self) -> Option<VitessVersion> {
        self.version
    }
}

impl PartialEq for VitessConnectorTag {
    fn eq(&self, other: &Self) -> bool {
        match (self.version, other.version) {
            (None, None) | (Some(_), None) | (None, Some(_)) => true,
            (Some(v1), Some(v2)) => v1 == v2,
        }
    }
}

impl FromStr for VitessVersion {
    type Err = TestError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let version = match s {
            "5.7" => Self::V5_7,
            "8.0" => Self::V8_0,
            _ => return Err(TestError::parse_error(format!("Unknown Vitess version `{s}`"))),
        };

        Ok(version)
    }
}

impl Display for VitessVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::V5_7 => write!(f, "5.7"),
            Self::V8_0 => write!(f, "8.0"),
        }
    }
}
