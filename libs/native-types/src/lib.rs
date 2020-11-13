//! This library aims to provide an exhaustive definition of all available native types for the databases Prisma supports.
//! There's one enum definition per database which lists all available types for the respective database.
mod error;
mod mssql;
mod mysql;
mod native_type;
mod postgres;
mod type_parameter;
mod parse;

pub use error::NativeTypeError;
pub use mssql::{MsSqlType, MsSqlTypeParameter};
pub use mysql::MySqlType;
pub use native_type::NativeType;
pub use postgres::PostgresType;
pub use type_parameter::NativeTypeParameter;
pub use parse::ParseTypeParameter;

pub(crate) type Result<T> = std::result::Result<T, NativeTypeError>;
