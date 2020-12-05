use crate::{MsSqlType, MySqlType, PostgresType};

#[derive(Debug, Clone, PartialEq)]
pub enum NativeType {
    MySQL(MySqlType),
    Postgres(PostgresType),
    MsSQL(MsSqlType),
}

impl NativeType {
    pub fn get_mysql_type(self) -> MySqlType {
        match self {
            NativeType::MySQL(tpe) => tpe,
            _ => panic!("Should only be called on Mysql."),
        }
    }

    pub fn get_mssql_type(self) -> MsSqlType {
        match self {
            NativeType::MsSQL(tpe) => tpe,
            _ => panic!("Should only be called on Mysql."),
        }
    }

    pub fn get_postgres_type(self) -> PostgresType {
        match self {
            NativeType::Postgres(tpe) => tpe,
            _ => panic!("Should only be called on Mysql."),
        }
    }
}
