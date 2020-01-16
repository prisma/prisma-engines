use crate::{SqlConnection, SyncSqlConnection};
use datamodel::{
    configuration::{MYSQL_SOURCE_NAME, POSTGRES_SOURCE_NAME, SQLITE_SOURCE_NAME},
    Source,
};
use quaint::{error::Error as QuaintError, prelude::*};
use tokio::runtime::Runtime;
use url::Url;

pub struct GenericSqlConnection {
    pool: Quaint,
    runtime: Runtime,
}

impl GenericSqlConnection {
    pub fn from_datasource(datasource: &dyn Source, db_name: Option<&str>) -> Result<Self, QuaintError> {
        let url = &datasource.url().value;

        let pool = match datasource.connector_type() {
            c if c == MYSQL_SOURCE_NAME => Quaint::new(url)?,
            c if c == POSTGRES_SOURCE_NAME => Quaint::new(url)?,
            c if c == SQLITE_SOURCE_NAME => Quaint::new(&Self::url_with_db(url, db_name))?,
            c => panic!("Unsuppored connectory type for SQL connection: {}", c),
        };

        Ok(Self {
            pool,
            runtime: super::default_runtime(),
        })
    }

    /// Create a pooled database connection. The `db_name` param is only used on SQLite if you want
    /// to provide a specific name for the bound database.
    pub fn from_database_str(url_str: &str, db_name: Option<&str>) -> Result<Self, QuaintError> {
        let url_parse_result: Result<Url, _> = url_str.parse();

        // Non-URL database strings are interpreted as SQLite file paths.
        if url_parse_result.is_err() {
            let pool = Quaint::new(&Self::url_with_db(&format!("file://{}", url_str), db_name))?;
            return Ok(Self {
                pool,
                runtime: super::default_runtime(),
            });
        }

        let url = url_parse_result?;

        let pool = match SqlFamily::from_scheme(url.scheme()) {
            Some(SqlFamily::Postgres) => Quaint::new(url_str)?,
            Some(SqlFamily::Mysql) => Quaint::new(url_str)?,
            Some(SqlFamily::Sqlite) => Quaint::new(&Self::url_with_db(url_str, db_name))?,
            None => panic!("Unsupported database URL scheme: {}", url.scheme()),
        };

        Ok(Self {
            pool,
            runtime: super::default_runtime(),
        })
    }

    pub fn connection_info(&self) -> &ConnectionInfo {
        self.pool.connection_info()
    }

    fn url_with_db(file_path: &str, db_name: Option<&str>) -> String {
        let db_name = db_name.unwrap_or("default-db-name");
        let mut splitted = file_path.split("?");
        let url = splitted.next().unwrap();
        let params = splitted.next();

        let mut params: Vec<&str> = match params {
            Some(params) => params.split("&").collect(),
            None => Vec::with_capacity(1),
        };

        let db_name_param = format!("db_name={}", db_name);
        params.push(&db_name_param);

        format!("{}?{}", url, params.join("&"))
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
        self.pool.query_raw(sql, params).await
    }
}

impl SyncSqlConnection for GenericSqlConnection {
    fn execute(&self, q: Query<'_>) -> Result<Option<Id>, QuaintError> {
        let conn = self.runtime.block_on(self.pool.check_out())?;
        self.runtime.block_on(conn.query(q))
    }

    fn query(&self, q: Query<'_>) -> Result<ResultSet, QuaintError> {
        let conn = self.runtime.block_on(self.pool.check_out())?;
        self.runtime.block_on(conn.query(q))
    }

    fn query_raw(&self, sql: &str, params: &[ParameterizedValue<'_>]) -> Result<ResultSet, QuaintError> {
        let conn = self.runtime.block_on(self.pool.check_out())?;
        self.runtime.block_on(conn.query_raw(sql, params))
    }

    fn execute_raw(&self, sql: &str, params: &[ParameterizedValue<'_>]) -> Result<u64, QuaintError> {
        let conn = self.runtime.block_on(self.pool.check_out())?;
        self.runtime.block_on(conn.query_raw(sql, params))
    }
}
