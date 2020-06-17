// use crate::{CoreError, ExpressionResult, OutputType, OutputTypeRef, QueryResult, QueryValue};
use indexmap::IndexMap;
// use internal::*;
use crate::PrismaError;
use failure::Fail;
use query_core::{
    response_ir::{Item, Map, ResponseData},
    CoreError,
};

#[derive(Debug, serde::Serialize, Default, PartialEq)]
pub struct GQLResponse {
    #[serde(skip_serializing_if = "IndexMap::is_empty")]
    data: Map,

    #[serde(skip_serializing_if = "Vec::is_empty")]
    errors: Vec<GQLError>,
}

#[derive(Debug, serde::Serialize, PartialEq)]
pub struct GQLError {
    error: String,
    user_facing_error: user_facing_errors::Error,
}

impl GQLResponse {
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            data: IndexMap::with_capacity(capacity),
            ..Default::default()
        }
    }

    pub fn insert_data(&mut self, key: impl Into<String>, item: Item) {
        self.data.insert(key.into(), item);
    }

    pub fn insert_error(&mut self, error: impl Into<GQLError>) {
        self.errors.push(error.into());
    }

    pub fn take_data(&mut self, key: impl AsRef<str>) -> Option<Item> {
        self.data.remove(key.as_ref())
    }
}

impl From<PrismaError> for GQLResponse {
    fn from(err: PrismaError) -> Self {
        let mut responses = Self::default();
        responses.insert_error(err);
        responses
    }
}

impl From<GQLError> for GQLResponse {
    fn from(err: GQLError) -> Self {
        let mut responses = Self::default();
        responses.insert_error(err);
        responses
    }
}

impl From<user_facing_errors::Error> for GQLResponse {
    fn from(err: user_facing_errors::Error) -> Self {
        let mut responses = Self::default();
        responses.insert_error(err);
        responses
    }
}

impl From<user_facing_errors::Error> for GQLError {
    fn from(err: user_facing_errors::Error) -> GQLError {
        GQLError {
            error: err.message().to_owned(),
            user_facing_error: err,
        }
    }
}

impl From<CoreError> for GQLError {
    fn from(err: CoreError) -> GQLError {
        GQLError {
            error: format!("{}", err),
            user_facing_error: err.into(),
        }
    }
}

impl From<PrismaError> for GQLError {
    fn from(other: PrismaError) -> Self {
        match other {
            PrismaError::CoreError(core_error) => GQLError::from(core_error),
            err => GQLError::from(user_facing_errors::Error::from_dyn_error(&err.compat())),
        }
    }
}

impl From<ResponseData> for GQLResponse {
    fn from(response: ResponseData) -> Self {
        let mut gql_response = GQLResponse::with_capacity(1);

        gql_response.insert_data(response.key, response.data);
        gql_response
    }
}

impl From<CoreError> for GQLResponse {
    fn from(err: CoreError) -> GQLResponse {
        let mut gql_response = GQLResponse::default();

        gql_response.insert_error(err);
        gql_response
    }
}
