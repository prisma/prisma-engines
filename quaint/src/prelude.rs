//! A "prelude" for users of the `quaint` crate.
pub use crate::ast::*;
pub use crate::connector::{
    ConnectionInfo, DefaultTransaction, ExternalConnectionInfo, Queryable, ResultRow, ResultSet, SqlFamily,
    TransactionCapable,
};
pub use crate::{col, val, values};

#[cfg(any(
    feature = "mssql-native",
    feature = "mysql-native",
    feature = "postgresql-native",
    feature = "sqlite-native"
))]
pub use crate::connector::NativeConnectionInfo;
