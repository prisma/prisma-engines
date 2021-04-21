use enumflags2::BitFlags;
use once_cell::sync::Lazy;
use std::error::Error as StdError;

#[derive(BitFlags, Copy, Clone, Debug, PartialEq)]
#[repr(u32)]
pub enum Tags {
    Mysql = 0x01,
    Mariadb = 0x02,
    Postgres = 0x04,
    Sqlite = 0x08,
    Mysql8 = 0x10,
    Mysql56 = 0x20,
    Mysql57 = 0x40,
    Mssql2017 = 0x80,
    Mssql2019 = 0x100,
    Postgres12 = 0x200,
    Mssql = 0x400,
    Vitess57 = 0x800,
}

impl Tags {
    pub fn empty() -> BitFlags<Tags> {
        BitFlags::empty()
    }

    pub fn from_name(name: &str) -> Result<BitFlags<Tags>, UnknownTagError> {
        TAG_NAMES
            .binary_search_by_key(&name, |(name, _tag)| *name)
            .ok()
            .and_then(|idx| TAG_NAMES.get(idx))
            .map(|(_name, tag)| *tag)
            .ok_or_else(|| UnknownTagError(name.to_owned()))
    }
}

#[derive(Debug)]
pub struct UnknownTagError(String);

impl std::fmt::Display for UnknownTagError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let available_tags: Vec<&str> = TAG_NAMES.iter().map(|(name, _)| *name).collect();
        write!(f, "Unknown tag `{}`. Available tags: {:?}", self.0, available_tags)
    }
}

impl StdError for UnknownTagError {}

/// All the tags, sorted by name.
static TAG_NAMES: Lazy<Vec<(&str, BitFlags<Tags>)>> = Lazy::new(|| {
    vec![
        ("mariadb", Tags::Mariadb.into()),
        ("mssql", Tags::Mssql.into()),
        ("mssql_2017", Tags::Mssql2017.into()),
        ("mssql_2019", Tags::Mssql2019.into()),
        ("mysql", Tags::Mysql.into()),
        ("mysql_5_6", Tags::Mysql56.into()),
        ("mysql_5_7", Tags::Mysql57.into()),
        ("mysql_8", Tags::Mysql8.into()),
        ("postgres", Tags::Postgres.into()),
        ("postgres_12", Tags::Postgres12.into()),
        ("sql", Tags::Mysql | Tags::Postgres | Tags::Sqlite | Tags::Mssql),
        ("sqlite", Tags::Sqlite.into()),
        ("vitess_5_7", Tags::Vitess57.into()),
    ]
});
