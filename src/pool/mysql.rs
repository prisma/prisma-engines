use super::PrismaConnectionManager;
use crate::{
    connector::{Mysql, MysqlParams, Queryable, metrics},
    error::Error,
};
use failure::{Compat, Fail};
use r2d2::ManageConnection;
use std::convert::TryFrom;

pub use mysql::OptsBuilder;
pub use r2d2_mysql::MysqlConnectionManager;

impl PrismaConnectionManager<MysqlConnectionManager> {
    pub fn mysql(opts: OptsBuilder) -> Self {
        Self {
            inner: MysqlConnectionManager::new(opts),
            file_path: None,
            schema: None,
        }
    }
}

impl TryFrom<MysqlParams> for r2d2::Pool<PrismaConnectionManager<MysqlConnectionManager>> {
    type Error = Error;

    fn try_from(params: MysqlParams) -> crate::Result<Self> {
        let manager = PrismaConnectionManager::mysql(params.config);

        let pool = r2d2::Pool::builder()
            .max_size(params.connection_limit)
            .build(manager)?;

        Ok(pool)
    }
}

impl ManageConnection for PrismaConnectionManager<MysqlConnectionManager> {
    type Connection = Mysql;
    type Error = Compat<Error>;

    fn connect(&self) -> Result<Self::Connection, Self::Error> {
        match metrics::connect("pool.mysql", || self.inner.connect()) {
            Ok(client) => Ok(Mysql::from(client)),
            Err(e) => Err(Error::from(e).compat()),
        }
    }

    fn is_valid(&self, conn: &mut Self::Connection) -> Result<(), Self::Error> {
        match conn.query_raw("SELECT version()", &[]) {
            Ok(_) => Ok(()),
            Err(e) => Err(e.compat()),
        }
    }

    fn has_broken(&self, _: &mut Self::Connection) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use url::Url;

    #[test]
    fn test_default_connection_limit() {
        let conn_string = format!(
            "mysql://{}:{}@{}:{}/{}",
            env::var("TEST_MYSQL_USER").unwrap(),
            env::var("TEST_MYSQL_PASSWORD").unwrap(),
            env::var("TEST_MYSQL_HOST").unwrap(),
            env::var("TEST_MYSQL_PORT").unwrap(),
            env::var("TEST_MYSQL_DB").unwrap(),
        );

        let url = Url::parse(&conn_string).unwrap();
        let params = MysqlParams::try_from(url).unwrap();
        let pool = r2d2::Pool::try_from(params).unwrap();

        assert_eq!(2, pool.max_size());
    }

    #[test]
    fn test_custom_connection_limit() {
        let conn_string = format!(
            "mysql://{}:{}@{}:{}/{}?connection_limit=10",
            env::var("TEST_MYSQL_USER").unwrap(),
            env::var("TEST_MYSQL_PASSWORD").unwrap(),
            env::var("TEST_MYSQL_HOST").unwrap(),
            env::var("TEST_MYSQL_PORT").unwrap(),
            env::var("TEST_MYSQL_DB").unwrap(),
        );

        let url = Url::parse(&conn_string).unwrap();
        let params = MysqlParams::try_from(url).unwrap();
        let pool = r2d2::Pool::try_from(params).unwrap();

        assert_eq!(10, pool.max_size());
    }
}
