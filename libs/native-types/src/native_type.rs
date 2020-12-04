use crate::{MsSqlType, MySqlType, PostgresType};

#[derive(Debug, Clone, PartialEq)]
pub enum NativeType {
    MySQL(MySqlType),
    Postgres(PostgresType),
    MsSQL(MsSqlType),
}
