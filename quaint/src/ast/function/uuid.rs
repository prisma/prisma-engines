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

/// Generates an optimized swapped UUID in MySQL 8
/// see `<https://dev.mysql.com/doc/refman/8.0/en/miscellaneous-functions.html#function_uuid-to-bin>`
/// ```rust
/// # use quaint::{ast::*, visitor::{Visitor, Mysql}};
/// # fn main() -> Result<(), quaint::error::Error> {
/// let query = Select::default().value(uuid_to_bin_swapped());
/// let (sql, _) = Mysql::build(query)?;
///
/// assert_eq!("SELECT uuid_to_bin(uuid(), 1)", sql);
/// # Ok(())
/// # }
/// ```
pub fn uuid_to_bin_swapped() -> Expression<'static> {
    let func = Function {
        typ_: FunctionType::UuidToBinSwapped,
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
#[cfg(feature = "mysql")]
pub fn native_uuid() -> Expression<'static> {
    let func = Function {
        typ_: FunctionType::Uuid,
        alias: None,
    };

    func.into()
}
