use super::*;
use crate::TestError;
use serde::{Deserialize, Serialize};

/// QueryParams enables parsing the generated id(s) of mutations sent to the Query Engine
/// so that it can be reused in subsequent queries
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryParams {
    selection: String,
    where_: QueryParamsWhere,
    where_many: QueryParamsWhereMany,
}

impl QueryParams {
    pub fn new<S>(selection: S, where_: QueryParamsWhere, where_many: QueryParamsWhereMany) -> Self
    where
        S: Into<String>,
    {
        QueryParams {
            selection: selection.into(),
            where_,
            where_many,
        }
    }

    /// Parses the JSON result of a mutation sent to the Query Engine in order to extract the generated id(s).
    /// Returns a string that's formatted to be included in another query. eg:
    /// "{ "id": "my_fancy_id" }"
    /// Equivalent of `.where()` in Scala
    pub fn parse(&self, json: serde_json::Value, path: &[&str]) -> Result<String, TestError> {
        let val = self.where_.parse(json, path)?;

        Ok(val)
    }

    /// Parses the JSON result of a mutation sent to the Query Engine in order to extract the generated id(s).
    /// Returns a string that's formatted to be included in another query. eg:
    /// "{ "id": "my_fancy_id" }"
    /// Equivalent of `.where()` in Scala
    pub fn parse_extend(&self, json: serde_json::Value, path: &[&str], meta: &str) -> Result<String, TestError> {
        let val = self.where_.parse_extend(json, path, meta)?;

        Ok(val)
    }

    /// Parses the JSON _array_ result of a mutation sent to the Query Engine in order to extract the generated id(s).
    /// Returns a Vec<String> where each id is formatted to be included in another query. eg:
    /// vec![{ "id": "my_fancy_id" }, { "id": "my_fancy_id_2" }]
    /// Equivalent of `.whereMulti()` in Scala
    pub fn parse_many(&self, json: serde_json::Value, path: &[&str]) -> Result<Vec<String>, TestError> {
        let val = self.where_many.parse(&json, path)?;

        Ok(val)
    }

    /// Parses the JSON _array_ result of a mutation sent to the Query Engine in order to extract the generated id(s).
    /// Returns the first id as a string that's formatted to be included in another query. eg:
    /// "{ "id": "my_fancy_id" }"
    /// Equivalent of `.whereFirst()` in Scala
    pub fn parse_many_first(&self, json: serde_json::Value, path: &[&str]) -> Result<String, TestError> {
        let val = self.where_many.parse(&json, path)?;

        Ok(val.first().unwrap().to_owned())
    }

    /// Parses the JSON _array_ result of a mutation sent to the Query Engine in order to extract the generated id(s).
    /// Returns all ids, formatted to be included in another query. eg:
    /// "[{ "id": "my_fancy_id" }, { "id": "my_fancy_id_2" }}"
    /// Equivalent of `.whereAll()` in Scala
    pub fn parse_many_all(&self, json: serde_json::Value, path: &[&str]) -> Result<String, TestError> {
        let val = self.where_many.parse(&json, path)?;

        Ok(format!("{}{}{}", "[", val.join(", "), "]"))
    }

    /// Get a reference to the query params's selection.
    pub fn selection(&self) -> &str {
        self.selection.as_str()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum QueryParamsWhere {
    Identifier(String),
    CompoundIdentifier(Vec<String>, String),
}

impl QueryParamsWhere {
    pub fn identifier(field: impl Into<String>) -> Self {
        Self::Identifier(field.into())
    }

    pub fn compound_identifier<V, F>(fields: V, arg_name: impl Into<String>) -> Self
    where
        F: Into<String>,
        V: Into<Vec<F>>,
    {
        QueryParamsWhere::CompoundIdentifier(fields.into().into_iter().map(|f| f.into()).collect(), arg_name.into())
    }

    pub fn parse(&self, json: serde_json::Value, path: &[&str]) -> Result<String, TestError> {
        match self {
            QueryParamsWhere::Identifier(field) => parse_id(field, &json, path, ""),
            QueryParamsWhere::CompoundIdentifier(fields, arg_name) => {
                parse_compound_id(fields, arg_name, &json, path, "")
            }
        }
    }

    pub fn parse_extend(&self, json: serde_json::Value, path: &[&str], meta: &str) -> Result<String, TestError> {
        match self {
            QueryParamsWhere::Identifier(field) => parse_id(field, &json, path, meta),
            QueryParamsWhere::CompoundIdentifier(fields, arg_name) => {
                parse_compound_id(fields, arg_name, &json, path, meta)
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum QueryParamsWhereMany {
    ManyIds(String),
    ManyCompounds(Vec<String>, String),
}

impl QueryParamsWhereMany {
    pub fn many_ids(field: impl Into<String>) -> Self {
        Self::ManyIds(field.into())
    }

    pub fn many_compounds<V, F>(fields: V, arg_name: impl Into<String>) -> Self
    where
        F: Into<String>,
        V: Into<Vec<F>>,
    {
        QueryParamsWhereMany::ManyCompounds(fields.into().into_iter().map(|f| f.into()).collect(), arg_name.into())
    }

    pub fn parse(&self, json: &serde_json::Value, path: &[&str]) -> Result<Vec<String>, TestError> {
        match self {
            QueryParamsWhereMany::ManyIds(field) => parse_many_ids(field, json, path),
            QueryParamsWhereMany::ManyCompounds(fields, arg_name) => {
                parse_many_compound_ids(fields, arg_name, json, path)
            }
        }
    }
}
