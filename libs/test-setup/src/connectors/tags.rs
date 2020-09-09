use enumflags2::BitFlags;
use once_cell::sync::Lazy;
use std::error::Error as StdError;

#[derive(BitFlags, Copy, Clone, Debug, PartialEq)]
#[repr(u8)]
pub enum Tags {
    Mysql = 0b0001,
    Mariadb = 0b0010,
    Postgres = 0b0100,
    Sqlite = 0b1000,
    Mysql8 = 0b00010000,
    Mysql56 = 0b00100000,
    Mssql2009 = 0b01000000,
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
        ("mssql_2019", Tags::Mssql2009.into()),
        ("mysql", Tags::Mysql.into()),
        ("mysql_5_6", Tags::Mysql56.into()),
        ("mysql_8", Tags::Mysql8.into()),
        ("postgres", Tags::Postgres.into()),
        ("sql", Tags::Mysql | Tags::Postgres | Tags::Sqlite),
        ("sqlite", Tags::Sqlite.into()),
    ]
});
