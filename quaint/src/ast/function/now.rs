use super::{Function, FunctionType};
use crate::ast::Expression;

/// Generates the SQL function NOW() returning the current timestamp in MySQL/PostgreSQL.
/// ```rust
/// # use quaint::{ast::*, visitor::{Visitor, Mysql, Postgres}};
/// # fn main() -> Result<(), quaint::error::Error> {
///
/// let query = Select::default().value(now());
///
/// let (sql, _) = Mysql::build(query)?;
/// assert_eq!("SELECT NOW()", sql);
///
/// let (sql, _) = Postgres::build(query)?;
/// assert_eq!("SELECT NOW()", sql);
/// # Ok(())
/// # }
/// ```
pub fn native_now() -> Expression<'static> {
    let func = Function {
        typ_: FunctionType::Now,
        alias: None,
    };

    func.into()
}
