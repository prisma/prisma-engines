use crate::HandlerError;
use indexmap::IndexMap;
use query_core::{
    query_graph_builder::QueryGraphBuilderError,
    response_ir::{Item, Map, ResponseData},
    CoreError, QueryParserError,
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

// FIXME: Prior to the introduction of structured validation errors, any HandlerError was an
// [user_facing_error::UnknownError].
//
// In addition, there's a specific variant of HandlerErrors, [HandlerError::Core] that wraps a
// CoreError. An existing implementation of From<CoreError> for a [user_facing_error::Error]
// converts some of the CoreError variants to specific user facing errors, but
// this method you are reading, was not using that implementation at all, and instead created
// [user_facing_errors::UnknownError] for every HandleError, including the Core variants
// that would could be converted to [user_facing_errors::KnownError] values.
//
// When adding the validation, we want those to be values of [user_facing::KnownError] type i.e.
// to have a proper error code and arbitrary metadata. We could leverage the From<CoreError> for
// [user_facing_error::Error], but that means that in addition to the validation errors, the client
// would see some other core errors to change the format, thus leading to a breaking change.
//
// The path forward would be to gradually:
//
// - Implement the UserFacingError trait for CoreError variants.
// - Refactor From<CoreError> to [user_facing_error::Error] to replace each variant case to be a no-op
// - Add a case clause in this trait's `from` function for each error that now has been migrating
//   and adapt the ts client to consume it, because now it will have a different format.
// - Once all variants have been refactored, change this method to instead reuse the From<CoreError>
//   trait implementation on [user_facing_error::Error].
// - Repeat the above strategy for other variantes of a [HandlerError] and if possible, flatten
//   the nested structure of variants.
impl From<HandlerError> for GQLError {
    fn from(err: HandlerError) -> Self {
        let user_facing: user_facing_errors::Error = match err {
            HandlerError::Core(ref ce) => match ce {
                CoreError::QueryParserError(QueryParserError::Structured(se))
                | CoreError::QueryGraphBuilderError(QueryGraphBuilderError::QueryParserError(
                    QueryParserError::Structured(se),
                )) => user_facing_errors::KnownError::new(se.to_owned()).into(),
                _ => user_facing_errors::UnknownError::new(&err).into(),
            },
            _ => user_facing_errors::UnknownError::new(&err).into(),
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
