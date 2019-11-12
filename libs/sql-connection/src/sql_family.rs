/// One of the supported SQL variants.
#[derive(Debug, PartialEq, Clone, Copy)]
pub enum SqlFamily {
    Postgres,
    Mysql,
    Sqlite,
}

impl SqlFamily {
    pub fn connector_type_string(&self) -> &'static str {
        match self {
            SqlFamily::Postgres => "postgresql",
            SqlFamily::Mysql => "mysql",
            SqlFamily::Sqlite => "sqlite",
        }
    }

    pub fn from_scheme(url_scheme: &str) -> Option<Self> {
        match url_scheme {
            "sqlite" | "file" => Some(SqlFamily::Sqlite),
            "postgres" | "postgresql" => Some(SqlFamily::Postgres),
            "mysql" => Some(SqlFamily::Mysql),
            _ => None,
        }
    }

    pub fn scheme_is_supported(url_scheme: &str) -> bool {
        Self::from_scheme(url_scheme).is_some()
    }
}
