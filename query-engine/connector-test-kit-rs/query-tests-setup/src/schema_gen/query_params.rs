use crate::{
    parse::{parse_compound_identifier, parse_identifier, parse_many_compounds, parse_many_ids},
    TestError,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryParams {
    selection: String,
    wher: QueryParamsWhere,
    where_many: QueryParamsWhereMany,
}

impl QueryParams {
    pub fn new<S>(selection: S, wher: QueryParamsWhere, where_many: QueryParamsWhereMany) -> Self
    where
        S: Into<String>,
    {
        QueryParams {
            selection: selection.into(),
            wher,
            where_many,
        }
    }

    pub fn where_first(&self, json: &serde_json::Value, path: &[&str]) -> Result<String, TestError> {
        let val = self.where_many.parse(json, path)?;

        Ok(val.get(0).unwrap().to_owned())
    }

    pub fn where_all(&self, json: &serde_json::Value, path: &[&str]) -> Result<String, TestError> {
        let val = self.where_many.parse(json, path)?;

        Ok(format!("{}{}{}", "[", val.join(", "), "]"))
    }

    /// Get a reference to the query params's selection.
    pub fn selection(&self) -> &str {
        self.selection.as_str()
    }

    /// Get a reference to the query params's wher.
    pub fn wher(&self) -> &QueryParamsWhere {
        &self.wher
    }

    /// Get a reference to the query params's where many.
    pub fn where_many(&self) -> &QueryParamsWhereMany {
        &self.where_many
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
            QueryParamsWhere::Identifier(field) => parse_identifier(field, &json, path),
            QueryParamsWhere::CompoundIdentifier(fields, arg_name) => {
                parse_compound_identifier(fields, arg_name, &json, path)
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
            QueryParamsWhereMany::ManyCompounds(fields, arg_name) => parse_many_compounds(fields, arg_name, json, path),
        }
    }
}
