use bitflags::bitflags;
use std::{error::Error as StdError, str::FromStr};

bitflags! {
    pub struct Tags: u8 {
        const POSTGRES      = 0b00000001;
        const SQLITE        = 0b00000010;
        const MSSQL         = 0b00000100;
        const MYSQL5_7      = 0b00001000;
        const MYSQL8        = 0b00010000;
        const MYSQL_MARIADB = 0b00100000;
    }
}

#[derive(Debug)]
pub struct UnknownTagError(String);

impl std::fmt::Display for UnknownTagError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let available_tags: Vec<&str> = tag_names().iter().map(|(name, _)| *name).collect();
        write!(f, "Unknown tag `{}`. Available tags: {:?}", self.0, available_tags)
    }
}

impl StdError for UnknownTagError {}

impl FromStr for Tags {
    type Err = UnknownTagError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let tags = tag_names();

        tags.binary_search_by_key(&s, |(name, _tag)| *name)
            .ok()
            .and_then(|idx| tags.get(idx))
            .map(|(_name, tag)| *tag)
            .ok_or_else(|| UnknownTagError(s.to_owned()))
    }
}

/// All the tags, sorted by name.
fn tag_names<'a>() -> [(&'a str, Tags); 7] {
    [
        ("mssql", Tags::MSSQL),
        ("mysql", Tags::MYSQL5_7 | Tags::MYSQL8 | Tags::MYSQL_MARIADB),
        ("mysql5_7", Tags::MYSQL5_7),
        ("mysql8", Tags::MYSQL8),
        ("mysql_mariadb", Tags::MYSQL_MARIADB),
        ("postgresql", Tags::POSTGRES),
        ("sqlite", Tags::SQLITE),
    ]
}
