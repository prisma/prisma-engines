//! This library aims to provide an exhaustive definition of all available native types for the databases Prisma supports.
//! There's one enum definition per database which lists all available types for the respective database.
mod error;
mod mssql;
mod mysql;
mod native_type;
mod postgres;
mod type_parameter;

pub use error::Error;
pub use mssql::MsSqlType;
pub use mysql::MySqlType;
pub use native_type::NativeType;
pub use postgres::PostgresType;
pub use type_parameter::TypeParameter;

pub(crate) type Result<T> = std::result::Result<T, error::Error>;
