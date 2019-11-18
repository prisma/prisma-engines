use crate::SqlConnection;
use datamodel::{
    configuration::{MYSQL_SOURCE_NAME, POSTGRES_SOURCE_NAME, SQLITE_SOURCE_NAME},
    Source,
};
use quaint::{error::Error as QuaintError, prelude::*};
use url::Url;

pub struct GenericSqlConnection {
    pool: Quaint,
}

impl GenericSqlConnection {
    pub fn from_datasource(datasource: &dyn Source, db_name: Option<&str>) -> Result<Self, QuaintError> {
        let url = &datasource.url().value;

        let pool = match datasource.connector_type() {
            c if c == MYSQL_SOURCE_NAME => Quaint::new(url)?,
            c if c == POSTGRES_SOURCE_NAME => Quaint::new(url)?,
            c if c == SQLITE_SOURCE_NAME => Quaint::new(&Self::url_with_db(url, db_name)?)?,
            c => panic!("Unsuppored connectory type for SQL connection: {}", c),
        };

        Ok(Self {
            pool,
        })
    }

    /// Create a pooled database connection. The `db_name` param is only used on SQLite if you want
    /// to provide a specific name for the bound database.
    pub fn from_database_str(url_str: &str, db_name: Option<&str>) -> Result<Self, QuaintError> {
        let url_parse_result: Result<Url, _> = url_str.parse();

        // Non-URL database strings are interpreted as SQLite file paths.
        if url_parse_result.is_err() {
            let pool = Quaint::new(&Self::url_with_db(&format!("file://{}", url_str), db_name)?)?;
            return Ok(Self {
                pool,
            });
        }

        let url = url_parse_result?;

        let pool = match SqlFamily::from_scheme(url.scheme()) {
            Some(SqlFamily::Postgres) => Quaint::new(url_str)?,
            Some(SqlFamily::Mysql) => Quaint::new(url_str)?,
            Some(SqlFamily::Sqlite) => Quaint::new(&Self::url_with_db(url_str, db_name)?)?,
            None => panic!("Unsupported database URL scheme: {}", url.scheme()),
        };

        Ok(Self {
            pool,
        })
    }

    pub fn connection_info(&self) -> &ConnectionInfo {
        self.pool.connection_info()
    }

    fn url_with_db(url: &str, db_name: Option<&str>) -> Result<String, QuaintError> {
        let mut url = Url::parse(url)?;
        url.query_pairs_mut().append_pair("db_name", db_name.unwrap_or("db"));
        Ok(url.as_str().to_string())
    }
}

#[async_trait::async_trait]
impl SqlConnection for GenericSqlConnection {
    async fn execute<'a>(&self, q: Query<'a>) -> Result<Option<Id>, QuaintError> {
        self.pool.execute(q).await
    }

    async fn query<'a>(&self, q: Query<'a>) -> Result<ResultSet, QuaintError> {
        self.pool.query(q).await
    }

    async fn query_raw<'a>(&self, sql: &str, params: &[ParameterizedValue<'a>]) -> Result<ResultSet, QuaintError> {
        self.pool.query_raw(sql, params).await
    }

    async fn execute_raw<'a>(&self, sql: &str, params: &[ParameterizedValue<'a>]) -> Result<u64, QuaintError> {
        self.pool.execute_raw(sql, params).await
    }
}
