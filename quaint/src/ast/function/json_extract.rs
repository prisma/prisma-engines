use super::Function;
use crate::ast::Expression;
use std::borrow::Cow;

#[derive(Debug, Clone, PartialEq)]
pub struct JsonExtract<'a> {
    pub(crate) column: Box<Expression<'a>>,
    pub(crate) path: JsonPath<'a>,
    pub(crate) extract_as_string: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JsonPath<'a> {
    #[cfg(feature = "mysql")]
    String(Cow<'a, str>),
    #[cfg(feature = "postgresql")]
    Array(Vec<Cow<'a, str>>),
}

impl<'a> JsonPath<'a> {
    #[cfg(feature = "mysql")]
    pub fn string<S>(string: S) -> JsonPath<'a>
    where
        S: Into<Cow<'a, str>>,
    {
        JsonPath::String(string.into())
    }

    #[cfg(feature = "postgresql")]
    pub fn array<A, V>(array: A) -> JsonPath<'a>
    where
        V: Into<Cow<'a, str>>,
        A: Into<Vec<V>>,
    {
        JsonPath::Array(array.into().into_iter().map(|v| v.into()).collect())
    }
}

/// Extracts a subset of a JSON blob given a path.
/// Two types of paths can be used:
/// - `String` paths, referring to JSON paths. This is supported by MySQL only.
/// - `Array` paths, supported by Postgres only.
///
/// For PostgreSQL:
/// ```rust
/// # use quaint::{ast::*, visitor::{Visitor, Postgres}};
/// # fn main() -> Result<(), quaint::error::Error> {
/// let extract: Expression = json_extract(Column::from(("users", "json")), JsonPath::array(["a", "b"]), false).into();
/// let query = Select::from_table("users").so_that(extract.equals("c"));
/// let (sql, params) = Postgres::build(query)?;
/// assert_eq!("SELECT \"users\".* FROM \"users\" WHERE (\"users\".\"json\"#>ARRAY[$1, $2]::text[])::jsonb = $3", sql);
/// assert_eq!(vec![Value::text("a"), Value::text("b"), Value::text("c")], params);
/// # Ok(())
/// # }
/// ```
/// For MySQL:
/// ```rust
/// # use quaint::{ast::*, visitor::{Visitor, Mysql}};
/// # fn main() -> Result<(), quaint::error::Error> {
/// let extract: Expression = json_extract(Column::from(("users", "json")), JsonPath::string("$.a.b"), false).into();
/// let query = Select::from_table("users").so_that(extract.equals("c"));
/// let (sql, params) = Mysql::build(query)?;
/// assert_eq!(r#"SELECT `users`.* FROM `users` WHERE (JSON_CONTAINS(JSON_EXTRACT(`users`.`json`, ?), ?) AND JSON_CONTAINS(?, JSON_EXTRACT(`users`.`json`, ?)))"#, sql);
/// assert_eq!(vec![Value::text("$.a.b"), Value::text("c"), Value::text("c"), Value::text("$.a.b")], params);
/// # Ok(())
/// # }
/// ```
pub fn json_extract<'a, C, P>(column: C, path: P, extract_as_string: bool) -> Function<'a>
where
    C: Into<Expression<'a>>,
    P: Into<JsonPath<'a>>,
{
    let fun = JsonExtract {
        column: Box::new(column.into()),
        path: path.into(),
        extract_as_string,
    };

    fun.into()
}
