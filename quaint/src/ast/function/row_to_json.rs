use super::Function;
use crate::ast::Table;

#[derive(Debug, Clone, PartialEq)]
#[cfg(feature = "postgresql")]
/// A representation of the `ROW_TO_JSON` function in the database.
/// Only for `Postgresql`
pub struct RowToJson<'a> {
    pub(crate) expr: Table<'a>,
    pub(crate) pretty_print: bool,
}

/// Return the given table in `JSON` format.
///
/// Only available for `Postgresql`
///
/// ```no_run
/// # use quaint::{ast::*, prelude::Queryable, visitor::{Visitor, Postgres}, single::Quaint, val};
/// # #[tokio::main]
/// # async fn main() -> Result<(), quaint::error::Error> {
/// # let conn = Quaint::new_in_memory()?;
/// let cte = Select::default()
///     .value(val!("hello_world").alias("toto"))
///     .into_cte("one");
///
/// let select = Select::from_table("one")
///     .value(row_to_json("one", false))
///     .with(cte);
///
/// let result = conn.select(select).await?;
///
/// assert_eq!(
///     Value::json(serde_json::json!({
///         "toto": "hello_world"
///     })),
///     result.into_single().unwrap()[0]
/// );
/// # Ok(())
/// # }
/// ```
#[cfg(feature = "postgresql")]
pub fn row_to_json<'a, T>(expr: T, pretty_print: bool) -> Function<'a>
where
    T: Into<Table<'a>>,
{
    let fun = RowToJson {
        expr: expr.into(),
        pretty_print,
    };

    fun.into()
}
