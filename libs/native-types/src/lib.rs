//! This library aims to provide an exhaustive definition of all available native types for the databases Prisma supports.
//! There's one enum definition per database which lists all available types for the respective database.
mod mssql;
mod mysql;
mod native_type;
mod postgres;

pub use mssql::MssqlType;
pub use mysql::MySqlType;
pub use native_type::NativeType;
pub use postgres::PostgresType;
