use crate::{
    ast::{Id, ParameterizedValue, Query},
    QueryResult,
};

pub trait ToResultRow {
    fn to_result_row<'b>(&'b self) -> QueryResult<ResultRow>;
}

pub trait Transaction {
    fn execute(&mut self, q: Query) -> QueryResult<Option<Id>>;
    fn query(&mut self, q: Query) -> QueryResult<Vec<ResultRow>>;
}

pub trait Transactional {
    fn with_transaction<F, T>(&self, db: &str, f: F) -> QueryResult<T>
    where
        F: FnOnce(&mut Transaction) -> QueryResult<T>;
}

#[derive(Debug, Default, PartialEq, Clone)]
pub struct ResultRow {
    pub values: Vec<ParameterizedValue>,
}
