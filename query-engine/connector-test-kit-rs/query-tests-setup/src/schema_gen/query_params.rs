use crate::parse::{parse_compound_identifier, parse_identifier, parse_multi, parse_multi_compound};

#[derive(Debug, Clone)]
pub struct QueryParams {
    selection: String,
    r#where: QueryParamsWhere,
    where_multi: QueryParamsMulti,
}

impl QueryParams {
    pub fn new<S>(selection: S, r#where: QueryParamsWhere, where_multi: QueryParamsMulti) -> Self
    where
        S: Into<String>,
    {
        QueryParams {
            selection: selection.into(),
            r#where,
            where_multi,
        }
    }

    pub fn where_first(&self, json: &serde_json::Value, path: &[String]) -> String {
        self.where_multi.parse(json, path).get(0).unwrap().to_owned()
    }

    pub fn where_all(&self, json: &serde_json::Value, path: &[String]) -> String {
        format!("{}{}{}", "[", self.where_multi.parse(json, path).join(", "), "]")
    }

    /// Get a reference to the query params's selection.
    pub fn selection(&self) -> &str {
        self.selection.as_str()
    }
}

#[derive(Debug, Clone)]
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

    pub fn parse(&self, json: &serde_json::Value, path: &[String]) -> String {
        match self {
            QueryParamsWhere::Identifier(field) => parse_identifier(field, json, path),
            QueryParamsWhere::CompoundIdentifier(fields, arg_name) => {
                parse_compound_identifier(fields, arg_name, json, path)
            }
        }
    }
}

#[derive(Debug, Clone)]
pub enum QueryParamsMulti {
    Multi(String),
    MultiCompound(Vec<String>, String),
}

impl QueryParamsMulti {
    pub fn multi(field: impl Into<String>) -> Self {
        Self::Multi(field.into())
    }

    pub fn multi_compound<V, F>(fields: V, arg_name: impl Into<String>) -> Self
    where
        F: Into<String>,
        V: Into<Vec<F>>,
    {
        QueryParamsMulti::MultiCompound(fields.into().into_iter().map(|f| f.into()).collect(), arg_name.into())
    }

    pub fn parse(&self, json: &serde_json::Value, path: &[String]) -> Vec<String> {
        match self {
            QueryParamsMulti::Multi(field) => parse_multi(field, json, path),
            QueryParamsMulti::MultiCompound(fields, arg_name) => parse_multi_compound(fields, arg_name, json, path),
        }
    }
}
