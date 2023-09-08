use crate::prelude::*;
use std::borrow::Cow;

#[derive(Debug, Clone, PartialEq)]
/// Holds the expressions on which to perform a full-text search
pub struct TextSearch<'a> {
    pub(crate) exprs: Vec<Expression<'a>>,
}

/// Performs a full-text search. Use it in combination with the `.matches()` comparable.
///
/// ```rust
/// # use quaint::{ast::*, visitor::{Visitor, Postgres}};
/// # fn main() -> Result<(), quaint::error::Error> {
/// let search: Expression = text_search(&[Column::from("name"), Column::from("ingredients")]).into();
/// let query = Select::from_table("recipes").so_that(search.matches("chicken"));
/// let (sql, params) = Postgres::build(query)?;
///
/// assert_eq!(
///    "SELECT \"recipes\".* FROM \"recipes\" \
///     WHERE to_tsvector(concat_ws(' ', \"name\",\"ingredients\")) @@ to_tsquery($1)", sql
/// );
///
/// assert_eq!(params, vec![Value::from("chicken")]);
/// # Ok(())    
/// # }
/// ```
#[cfg(any(feature = "postgresql", feature = "mysql"))]
pub fn text_search<'a, T: Clone>(exprs: &[T]) -> super::Function<'a>
where
    T: Into<Expression<'a>>,
{
    let exprs: Vec<Expression> = exprs.iter().map(|c| c.clone().into()).collect();
    let fun = TextSearch { exprs };

    fun.into()
}

#[derive(Debug, Clone, PartialEq)]
/// Holds the expressions & query on which to perform a text-search ranking compute
pub struct TextSearchRelevance<'a> {
    pub(crate) exprs: Vec<Expression<'a>>,
    pub(crate) query: Cow<'a, str>,
}

/// Computes the relevance score of a full-text search query against some expressions.
///
/// ```rust
/// # use quaint::{ast::*, visitor::{Visitor, Postgres}};
/// # fn main() -> Result<(), quaint::error::Error> {
/// let relevance: Expression = text_search_relevance(&[Column::from("name"), Column::from("ingredients")], "chicken").into();
/// let query = Select::from_table("recipes").so_that(relevance.greater_than(0.1));
/// let (sql, params) = Postgres::build(query)?;
///
/// assert_eq!(
///    "SELECT \"recipes\".* FROM \"recipes\" WHERE \
///     ts_rank(to_tsvector(concat_ws(' ', \"name\",\"ingredients\")), to_tsquery($1)) > $2", sql
/// );
///
/// assert_eq!(params, vec![Value::from("chicken"), Value::from(0.1)]);
/// # Ok(())    
/// # }
/// ```
#[cfg(any(feature = "postgresql", feature = "mysql"))]
pub fn text_search_relevance<'a, E: Clone, Q>(exprs: &[E], query: Q) -> super::Function<'a>
where
    E: Into<Expression<'a>>,
    Q: Into<Cow<'a, str>>,
{
    let exprs: Vec<Expression> = exprs.iter().map(|c| c.clone().into()).collect();
    let fun = TextSearchRelevance {
        exprs,
        query: query.into(),
    };

    fun.into()
}
