/// Test-relevant connector tags.
#[enumflags2::bitflags]
#[derive(Copy, Clone, Debug, PartialEq)]
#[repr(u16)]
pub enum Tags {
    LowerCasesTableNames = 1 << 0,
    Mysql = 1 << 1,
    Mariadb = 1 << 2,
    Postgres = 1 << 3,
    Sqlite = 1 << 4,
    Mysql8 = 1 << 5,
    Mysql56 = 1 << 6,
    Mysql57 = 1 << 7,
    Mssql2017 = 1 << 8,
    Mssql2019 = 1 << 9,
    Postgres12 = 1 << 10,
    Mssql = 1 << 11,
    Vitess = 1 << 12,
    Cockroach = 1 << 13,
    Postgres14 = 1 << 14,
}
