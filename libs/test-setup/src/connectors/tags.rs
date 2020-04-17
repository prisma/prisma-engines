use bitflags::bitflags;

bitflags! {
    pub struct Tags: u8 {
        const MYSQL     = 0b00000001;
        const MARIADB   = 0b00000010;
        const POSTGRES  = 0b00000100;
        const SQLITE    = 0b00001000;
        const MYSQL_8   = 0b00010000;
        const MYSQL_5_6 = 0b00100000;

        const SQL = Self::MYSQL.bits | Self::POSTGRES.bits | Self::SQLITE.bits;
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

impl std::error::Error for UnknownTagError {}

impl std::str::FromStr for Tags {
    type Err = UnknownTagError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        TAG_NAMES
            .binary_search_by_key(&s, |(name, _tag)| *name)
            .ok()
            .and_then(|idx| TAG_NAMES.get(idx))
            .map(|(_name, tag)| *tag)
            .ok_or_else(|| UnknownTagError(s.to_owned()))
    }
}

/// All the tags, sorted by name.
const TAG_NAMES: &[(&str, Tags)] = &[
    ("mariadb", Tags::MARIADB),
    ("mysql", Tags::MYSQL),
    ("mysql_5_6", Tags::MYSQL_5_6),
    ("mysql_8", Tags::MYSQL_8),
    ("postgres", Tags::POSTGRES),
    ("sql", Tags::SQL),
    ("sqlite", Tags::SQLITE),
];
