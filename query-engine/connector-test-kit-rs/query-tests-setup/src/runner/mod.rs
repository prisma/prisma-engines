mod binary;
mod direct;
mod napi;

pub use binary::*;
pub use direct::*;
pub use napi::*;

use crate::{QueryResult, TestError, TestResult};

pub trait RunnerInterface {}

pub enum Runner {
    /// Using the QE crate directly for queries.
    Direct(DirectRunner),

    /// Using a NodeJS runner.
    NApi(NApiRunner),

    /// Using the HTTP bridge
    Binary(BinaryRunner),
}

impl Runner {
    pub fn load(ident: &str, datamodel: String) -> TestResult<Self> {
        match ident {
            "direct" => Ok(Self::Direct(DirectRunner {})),
            "napi" => Ok(Self::NApi(NApiRunner {})),
            "binary" => Ok(Self::Binary(BinaryRunner {})),
            unknown => Err(TestError::parse_error(format!("Unknown test runner '{}'", unknown))),
        }
    }

    pub fn query<T>(&self, _gql: T) -> QueryResult
    where
        T: Into<String>,
    {
        todo!()
    }

    pub fn batch<T>(&self, _gql: T) -> QueryResult
    where
        T: Into<String>,
    {
        todo!()
    }
}
