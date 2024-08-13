use std::fmt::Display;

use query_core::constants::custom_types;
use request_handlers::{GQLError, PrismaResponse};
use serde::{Deserialize, Serialize};
#[derive(Serialize, Deserialize, Debug, PartialEq)]
struct SimpleGqlErrorResponse {
    #[serde(skip_serializing_if = "Vec::is_empty")]
    errors: Vec<GQLError>,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
struct SimpleGqlResponse {
    #[serde(skip_serializing_if = "SimpleGqlResponse::data_is_empty")]
    #[serde(default)]
    data: serde_json::Value,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    #[serde(default)]
    errors: Vec<GQLError>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    extensions: Option<serde_json::Value>,
}

impl SimpleGqlResponse {
    fn data_is_empty(data: &serde_json::Value) -> bool {
        match data {
            serde_json::Value::Object(o) => o.is_empty(),
            serde_json::Value::Null => true,
            _ => false,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
struct SimpleGqlBatchResponse {
    batch_result: Vec<SimpleGqlResponse>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    #[serde(default)]
    errors: Vec<GQLError>,
    #[serde(skip_serializing_if = "Option::is_none")]
    extensions: Option<serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
enum Response {
    Error(SimpleGqlErrorResponse),
    Multi(SimpleGqlBatchResponse),
    Single(SimpleGqlResponse),
}

#[derive(Debug, Deserialize, PartialEq)]
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
            panic!("Expected result to return an error, but found success: {self}");
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
                    self
                );
            } else {
                panic!(
                    "Expected error with code `{}`, got: `{}`",
                    err_code.unwrap_or_else(|| "None".to_owned()),
                    self
                );
            }
        }
    }

    pub fn errors(&self) -> Vec<&GQLError> {
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

impl Display for QueryResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&serde_json::to_value(&self.response).unwrap().to_string())
    }
}

impl From<PrismaResponse> for QueryResult {
    fn from(response: PrismaResponse) -> Self {
        match response {
            PrismaResponse::Single(res) => QueryResult {
                response: Response::Single(SimpleGqlResponse {
                    data: serde_json::to_value(res.data).unwrap(),
                    errors: res.errors,
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
                            errors: res.errors,
                            extensions: (!res.extensions.is_empty())
                                .then(|| serde_json::to_value(&res.extensions).unwrap()),
                        })
                        .collect(),
                    errors: reses.errors,
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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_deserializing_successful_batch_response() {
        let response = "{\"batchResult\":[{\"data\":{\"findUniqueTestModelOrThrow\":{\"id\":1}}},{\"data\":{\"findUniqueTestModelOrThrow\":{\"id\":2}}}]}";
        let result: QueryResult = serde_json::from_str(response).unwrap();

        let expected = QueryResult {
            response: Response::Multi(SimpleGqlBatchResponse {
                batch_result: vec![
                    SimpleGqlResponse {
                        data: json!({
                            "findUniqueTestModelOrThrow": {
                                "id": 1,
                            },
                        }),
                        errors: vec![],
                        extensions: None,
                    },
                    SimpleGqlResponse {
                        data: json!({
                            "findUniqueTestModelOrThrow": {
                                "id": 2,
                            },
                        }),
                        errors: vec![],
                        extensions: None,
                    },
                ],
                errors: vec![],
                extensions: None,
            }),
        };
        assert_eq!(result, expected);
    }

    #[test]
    fn test_deserializing_error_batch_response() {
        let response = r#"
{
   "batchResult":[
      {
         "data":{
            "findUniqueTestModelOrThrow":{
               "id":2
            }
         }
      },
      {
         "errors":[
            {
               "error":"An operation failed because it depends on one or more records that were required but not found. Expected a record, found none.",
               "user_facing_error":{
                  "is_panic":false,
                  "message":"An operation failed because it depends on one or more records that were required but not found. Expected a record, found none.",
                  "meta":{
                     "cause":"Expected a record, found none."
                  },
                  "error_code":"P2025"
               }
            }
         ]
      }
   ]
}"#;
        let result: QueryResult = serde_json::from_str(response).unwrap();

        let expected = QueryResult {
            response: Response::Multi(SimpleGqlBatchResponse {
                batch_result: vec![
                    SimpleGqlResponse {
                        data: json!({"findUniqueTestModelOrThrow": {"id": 2}}),
                        errors: vec![],
                        extensions: None,
                    },
                    SimpleGqlResponse {
                        data: serde_json::Value::Null,
                        errors: vec![GQLError::from_user_facing_error(user_facing_errors::KnownError {
                            message: "An operation failed because it depends on one or more records that were required but not found. Expected a record, found none.".to_string(),
                            meta: json!({"cause": "Expected a record, found none."}),
                            error_code: std::borrow::Cow::from("P2025"),
                        }.into())],
                        extensions: None,
                    },
                ],
                errors: vec![],
                extensions: None,
            }),
        };
        assert_eq!(result, expected);
    }
}
