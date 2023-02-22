use crate::HandlerError;
use indexmap::IndexMap;
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

    #[serde(skip_serializing_if = "IndexMap::is_empty")]
    extensions: Map,
}

#[derive(Debug, serde::Serialize, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct GQLBatchResponse {
    #[serde(skip_serializing_if = "Vec::is_empty")]
    batch_result: Vec<GQLResponse>,

    #[serde(skip_serializing_if = "Vec::is_empty")]
    errors: Vec<GQLError>,

    #[serde(skip_serializing_if = "IndexMap::is_empty")]
    extensions: Map,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct GQLError {
    error: String,
    user_facing_error: user_facing_errors::Error,
}

impl GQLError {
    pub fn code(&self) -> Option<&str> {
        self.user_facing_error.as_known().map(|err| err.error_code.as_ref())
    }

    pub fn message(&self) -> &str {
        self.user_facing_error.message()
    }

    pub fn batch_request_idx(&self) -> Option<usize> {
        self.user_facing_error.batch_request_idx()
    }
}

impl GQLResponse {
    pub fn new(data: Map) -> Self {
        Self {
            data,
            ..Default::default()
        }
    }

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

    pub fn errors(&self) -> impl Iterator<Item = &GQLError> {
        self.errors.iter()
    }

    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }

    pub fn into_data(self) -> Map {
        self.data
    }

    pub fn set_extension(&mut self, key: String, val: serde_json::Value) {
        self.extensions.entry(key).or_insert(Item::Json(val));
    }
}

impl From<HandlerError> for GQLResponse {
    fn from(err: HandlerError) -> Self {
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

impl From<HandlerError> for GQLError {
    fn from(err: HandlerError) -> Self {
        let user_facing = match err {
            HandlerError::Core(ce) => user_facing_errors::Error::from(ce),
            _ => user_facing_errors::Error::to_unknown(&err),
        };
        GQLError::from(user_facing)
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
            error: format!("{err}"),
            user_facing_error: err.into(),
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

/// GQLBatchResponse converters

impl GQLBatchResponse {
    pub fn insert_responses(&mut self, responses: Vec<GQLResponse>) {
        responses.into_iter().for_each(|response| {
            self.batch_result.push(response);
        })
    }

    pub fn insert_error(&mut self, error: impl Into<GQLError>) {
        self.errors.push(error.into());
    }

    pub fn errors(&self) -> impl Iterator<Item = &GQLError> {
        self.errors
            .iter()
            .chain(self.batch_result.iter().flat_map(|res| res.errors()))
    }

    pub fn into_responses(self) -> Vec<GQLResponse> {
        self.batch_result
    }

    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty() || self.batch_result.iter().any(|res| res.has_errors())
    }

    pub fn set_extension(&mut self, key: String, val: serde_json::Value) {
        self.extensions.entry(key).or_insert(Item::Json(val));
    }
}

impl From<user_facing_errors::Error> for GQLBatchResponse {
    fn from(err: user_facing_errors::Error) -> Self {
        let mut batch_response = Self::default();
        batch_response.insert_error(err);
        batch_response
    }
}

impl From<CoreError> for GQLBatchResponse {
    fn from(err: CoreError) -> Self {
        let mut batch_response = Self::default();

        batch_response.insert_error(err);
        batch_response
    }
}

impl From<HandlerError> for GQLBatchResponse {
    fn from(err: HandlerError) -> Self {
        let mut responses = Self::default();
        responses.insert_error(err);
        responses
    }
}

impl From<Vec<GQLResponse>> for GQLBatchResponse {
    fn from(responses: Vec<GQLResponse>) -> Self {
        let mut batch_response = Self::default();

        batch_response.insert_responses(responses);
        batch_response
    }
}
