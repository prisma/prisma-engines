use request_handlers::{GQLError, PrismaResponse};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
struct SimpleGqlResponse {
    data: serde_json::Value,
    errors: Vec<GQLError>,
}

#[derive(Serialize, Deserialize, Debug)]
struct SimpleGqlBatchResponse {
    batch_result: Vec<SimpleGqlResponse>,
    errors: Vec<GQLError>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
enum Response {
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
                Some(msg) => err.message().contains(msg),
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

    pub fn errors(&self) -> Vec<&GQLError> {
        match self.response {
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
                    errors: res.errors,
                }),
            },
            PrismaResponse::Multi(reses) => QueryResult {
                response: Response::Multi(SimpleGqlBatchResponse {
                    batch_result: reses
                        .batch_result
                        .into_iter()
                        .map(|res| SimpleGqlResponse {
                            data: serde_json::to_value(&response.data).unwrap(),
                            errors: res.errors,
                        })
                        .collect(),
                    errors: reses.errors,
                }),
            },
        }
    }
}
