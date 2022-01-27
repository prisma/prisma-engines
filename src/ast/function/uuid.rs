use super::{Function, FunctionType};
use crate::ast::Expression;

/// Generates the function uuid_to_bin(uuid()) returning a binary uuid in MySQL
/// ```rust
/// # use quaint::{ast::*, visitor::{Visitor, Mysql}};
/// # fn main() -> Result<(), quaint::error::Error> {

/// let query = Select::default().value(uuid_to_bin());
/// let (sql, _) = Mysql::build(query)?;
///
/// assert_eq!("SELECT uuid_to_bin(uuid())", sql);
/// # Ok(())
/// # }
/// ```
#[cfg(feature = "mysql")]
pub fn uuid_to_bin() -> Expression<'static> {
    let func = Function {
        typ_: FunctionType::UuidToBin,
        alias: None,
    };

    func.into()
}

/// Generates the function uuid_to_bin(uuid()) returning a binary uuid in MySQL
/// ```rust
/// # use quaint::{ast::*, visitor::{Visitor, Mysql}};
/// # fn main() -> Result<(), quaint::error::Error> {

/// let query = Select::default().value(native_uuid());
/// let (sql, _) = Mysql::build(query)?;
///
/// assert_eq!("SELECT uuid()", sql);
/// # Ok(())
/// # }
/// ```
#[cfg(any(feature = "mysql"))]
pub fn native_uuid() -> Expression<'static> {
    let func = Function {
        typ_: FunctionType::Uuid,
        alias: None,
    };

    func.into()
}
