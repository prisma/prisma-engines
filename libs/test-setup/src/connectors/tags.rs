use bitflags::bitflags;

bitflags! {
    pub struct Tags: u8 {
        const MYSQL    = 0b00000001;
        const MARIADB  = 0b00000010;
        const POSTGRES = 0b00000100;
        const SQLITE   = 0b00001000;

        const SQL = Self::MYSQL.bits | Self::POSTGRES.bits | Self::SQLITE.bits;
    }
}

#[derive(Debug)]
pub struct UnknownTagError(String);

impl std::fmt::Display for UnknownTagError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Unknown tag `{}`.", self.0)
    }
}

impl std::error::Error for UnknownTagError {}

impl std::str::FromStr for Tags {
    type Err = UnknownTagError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "mysql" => Ok(Tags::MYSQL),
            "postgres" => Ok(Tags::POSTGRES),
            "sqlite" => Ok(Tags::SQLITE),
            "sql" => Ok(Tags::SQL),
            "mariadb" => Ok(Tags::MARIADB),
            _ => Err(UnknownTagError(s.to_owned())),
        }
    }
}
