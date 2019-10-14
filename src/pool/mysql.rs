pub use mysql_async::OptsBuilder;

use crate::{
    connector::{Mysql, Queryable, DBIO},
    error::Error,
};
use tokio_resource_pool::{Manage, Status, RealDependencies, CheckOut};
use futures::future;

pub struct MysqlManager {
    opts: OptsBuilder,
}

impl MysqlManager {
    pub fn new(opts: OptsBuilder) -> Self {
        Self { opts }
    }
}

impl Manage for MysqlManager {
    type Resource = Mysql;
    type Dependencies = RealDependencies;
    type CheckOut = CheckOut<Self>;
    type Error = Error;
    type CreateFuture = DBIO<'static, Self::Resource>;
    type RecycleFuture = DBIO<'static, Option<Self::Resource>>;

    fn create(&self) -> Self::CreateFuture {
        DBIO::new(match Mysql::new(self.opts.clone()) {
            Ok(mysql) => future::ok(mysql),
            Err(e) => future::err(e),
        })
    }

    fn status(&self, _: &Self::Resource) -> Status {
        Status::Valid
    }

    fn recycle(&self, connection: Self::Resource) -> Self::RecycleFuture {
        DBIO::new(async {
            connection.query_raw("", &[]).await?;
            Ok(Some(connection))
        })
    }
}

#[cfg(test)]
mod tests {
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
        let pool = crate::pool::mysql(url).unwrap();

        assert_eq!(num_cpus::get_physical() * 2 + 1, pool.capacity());
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
        let pool = crate::pool::mysql(url).unwrap();

        assert_eq!(10, pool.capacity());
    }
}
