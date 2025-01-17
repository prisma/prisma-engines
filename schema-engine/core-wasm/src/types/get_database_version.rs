//! Get the database version for error reporting.

pub struct GetDatabaseVersionInput {
    pub datasource: DatasourceParam,
}

pub struct GetDatabaseVersionOutput {
    pub version: String,
}
