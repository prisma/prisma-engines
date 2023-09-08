use query_core::constants::custom_types;
use request_handlers::PrismaResponse;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct SimpleGqlError {
    error: String,
    #[serde(default)]
    meta: Option<serde_json::Value>,
    #[serde(default)]
    user_facing_error: Option<serde_json::Value>,
}

impl SimpleGqlError {
    fn code(&self) -> Option<&str> {
        self.user_facing_error.as_ref()?["error_code"].as_str()
    }

    fn message(&self) -> &str {
        self.user_facing_error
            .as_ref()
            .and_then(|err| err["message"].as_str())
            .unwrap_or(&self.error)
    }

    pub fn batch_request_idx(&self) -> Option<usize> {
        todo!()
    }
}

#[derive(Serialize, Deserialize, Debug)]
struct SimpleGqlErrorResponse {
    #[serde(skip_serializing_if = "Vec::is_empty")]
    errors: Vec<SimpleGqlError>,
}

#[derive(Serialize, Deserialize, Debug)]
struct SimpleGqlResponse {
    data: serde_json::Value,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    #[serde(default)]
    errors: Vec<SimpleGqlError>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    extensions: Option<serde_json::Value>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct SimpleGqlBatchResponse {
    batch_result: Vec<SimpleGqlResponse>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    #[serde(default)]
    errors: Vec<SimpleGqlError>,
    #[serde(skip_serializing_if = "Option::is_none")]
    extensions: Option<serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
enum Response {
    Error(SimpleGqlErrorResponse),
    Single(SimpleGqlResponse),
    Multi(SimpleGqlBatchResponse),
}

#[derive(Debug, Deserialize)]
#[serde(transparent)]
pub struct QueryResult {
    response: Response,
}

impl QueryResult {
    pub fn failed(&self) -> bool {
        match self.response {
            Response::Error(ref s) => !s.errors.is_empty(),
            Response::Single(ref s) => !s.errors.is_empty(),
            Response::Multi(ref m) => !(m.errors.is_empty() && m.batch_result.iter().all(|res| res.errors.is_empty())),
        }
    }

    /// Asserts absence of errors in the result. Panics with assertion error.
    pub fn assert_success(&self) {
        if self.failed() {
            panic!("{}", self.to_string());
        }
    }

    /// Asserts presence of errors in the result.
    /// Code must equal the given one, the message is a partial match.
    /// If more than one error is contained, asserts that at least one error contains the message _and_ code.
    ///
    /// Panics with assertion error on no match.
    pub fn assert_failure(&self, err_code: impl Into<Option<usize>>, msg_contains: Option<String>) {
        let err_code: Option<usize> = err_code.into();
        if !self.failed() {
            panic!(
                "Expected result to return an error, but found success: {}",
                self.to_string()
            );
        }

        // 0 is the "do nothing marker"
        if err_code == Some(0) {
            return;
        }

        let err_code = err_code.map(|code| format!("P{code}"));
        let err_exists = self.errors().into_iter().any(|err| {
            let code_matches = err.code() == err_code.as_deref();
            let msg_matches = match msg_contains.as_ref() {
                Some(msg) => dbg!(err.message()).contains(msg),
                None => true,
            };

            code_matches && msg_matches
        });

        if !err_exists {
            if let Some(msg) = msg_contains {
                panic!(
                    "Expected error with code `{}` and message `{}`, got: `{}`",
                    err_code.unwrap_or_else(|| "None".to_owned()),
                    msg,
                    self.to_string()
                );
            } else {
                panic!(
                    "Expected error with code `{}`, got: `{}`",
                    err_code.unwrap_or_else(|| "None".to_owned()),
                    self.to_string()
                );
            }
        }
    }

    pub fn errors(&self) -> Vec<&SimpleGqlError> {
        match self.response {
            Response::Error(ref s) => s.errors.iter().collect(),
            Response::Single(ref s) => s.errors.iter().collect(),
            Response::Multi(ref m) => m
                .errors
                .iter()
                .chain(m.batch_result.iter().flat_map(|res| res.errors.iter()))
                .collect(),
        }
    }

    pub fn to_json_value(&self) -> serde_json::Value {
        serde_json::to_value(&self.response).unwrap()
    }

    pub fn to_string_pretty(&self) -> String {
        serde_json::to_string_pretty(&self.response).unwrap()
    }

    /// Transform a JSON protocol response to a GraphQL protocol response, by removing the type
    /// tags.
    pub(crate) fn detag(&mut self) {
        match &mut self.response {
            Response::Error(_) => (),
            Response::Single(res) => detag_value(&mut res.data),
            Response::Multi(res) => {
                for res in &mut res.batch_result {
                    detag_value(&mut res.data)
                }
            }
        }
    }
}

impl ToString for QueryResult {
    fn to_string(&self) -> String {
        serde_json::to_value(&self.response).unwrap().to_string()
    }
}

impl From<PrismaResponse> for QueryResult {
    fn from(response: PrismaResponse) -> Self {
        match response {
            PrismaResponse::Single(res) => QueryResult {
                response: Response::Single(SimpleGqlResponse {
                    data: serde_json::to_value(res.data).unwrap(),
                    errors: res.errors.into_iter().map(|err| convert_gql_error(&err)).collect(),
                    extensions: (!res.extensions.is_empty()).then(|| serde_json::to_value(&res.extensions).unwrap()),
                }),
            },
            PrismaResponse::Multi(reses) => QueryResult {
                response: Response::Multi(SimpleGqlBatchResponse {
                    batch_result: reses
                        .batch_result
                        .into_iter()
                        .map(|res| SimpleGqlResponse {
                            data: serde_json::to_value(&res.data).unwrap(),
                            errors: res.errors.into_iter().map(|err| convert_gql_error(&err)).collect(),
                            extensions: (!res.extensions.is_empty())
                                .then(|| serde_json::to_value(&res.extensions).unwrap()),
                        })
                        .collect(),
                    errors: reses.errors.into_iter().map(|err| convert_gql_error(&err)).collect(),
                    extensions: (!reses.extensions.is_empty())
                        .then(|| serde_json::to_value(&reses.extensions).unwrap()),
                }),
            },
        }
    }
}

fn detag_value(val: &mut serde_json::Value) {
    match val {
        serde_json::Value::Object(obj) => {
            if obj.len() == 2 && obj.contains_key(custom_types::TYPE) && obj.contains_key(custom_types::VALUE) {
                let mut new_val = obj.remove(custom_types::VALUE).unwrap();
                detag_value(&mut new_val);
                *val = new_val;
            } else {
                for elem in obj.values_mut() {
                    detag_value(elem);
                }
            }
        }
        serde_json::Value::Array(arr) => {
            for elem in arr {
                detag_value(elem)
            }
        }
        _ => (),
    }
}

fn convert_gql_error(err: &request_handlers::GQLError) -> SimpleGqlError {
    SimpleGqlError {
        error: err.message().to_owned(),
        meta: err.code().map(|code| {
            serde_json::json!({
                "code": code,
                "message": err.message()
            })
        }),
        user_facing_error: err.code().map(|code| {
            serde_json::json!({
                "code": code,
                "message": err.message()
            })
        }),
    }
}
