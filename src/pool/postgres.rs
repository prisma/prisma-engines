use crate::{
    connector::{postgres::SslParams, PostgreSql, Queryable, DBIO},
    error::Error,
};
use tokio_postgres::Config;
use tokio_resource_pool::{CheckOut, Manage, RealDependencies, Status};

pub struct PostgresManager {
    config: Config,
    schema: Option<String>,
    ssl_params: Option<SslParams>,
}

impl PostgresManager {
    pub fn new(config: Config, schema: Option<String>, ssl_params: Option<SslParams>) -> Self {
        Self {
            config,
            schema,
            ssl_params,
        }
    }
}

impl Manage for PostgresManager {
    type Resource = PostgreSql;
    type Dependencies = RealDependencies;
    type CheckOut = CheckOut<Self>;
    type Error = Error;
    type CreateFuture = DBIO<'static, Self::Resource>;
    type RecycleFuture = DBIO<'static, Option<Self::Resource>>;

    fn create(&self) -> Self::CreateFuture {
        let config = self.config.clone();
        let schema = self.schema.clone();
        let ssl_params = self.ssl_params.clone();

        DBIO::new(async move { PostgreSql::new(config, schema, ssl_params).await })
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
    use super::*;
    use std::env;
    use url::Url;

    #[test]
    fn test_default_connection_limit() {
        let conn_string = format!(
            "postgresql://{}:{}@{}:{}/{}",
            env::var("TEST_PG_USER").unwrap(),
            env::var("TEST_PG_PASSWORD").unwrap(),
            env::var("TEST_PG_HOST").unwrap(),
            env::var("TEST_PG_PORT").unwrap(),
            env::var("TEST_PG_DB").unwrap(),
        );

        let url = Url::parse(&conn_string).unwrap();
        let pool = crate::pool::postgres(url).unwrap();

        assert_eq!(num_cpus::get_physical() * 2 + 1, pool.capacity());
    }

    #[test]
    fn test_custom_connection_limit() {
        let conn_string = format!(
            "postgresql://{}:{}@{}:{}/{}?connection_limit=10",
            env::var("TEST_PG_USER").unwrap(),
            env::var("TEST_PG_PASSWORD").unwrap(),
            env::var("TEST_PG_HOST").unwrap(),
            env::var("TEST_PG_PORT").unwrap(),
            env::var("TEST_PG_DB").unwrap(),
        );

        let url = Url::parse(&conn_string).unwrap();
        let pool = crate::pool::postgres(url).unwrap();

        assert_eq!(10, pool.capacity());
    }

    #[tokio::test]
    async fn test_custom_search_path() {
        let conn_string = format!(
            "postgresql://{}:{}@{}:{}/{}?schema=musti-test",
            env::var("TEST_PG_USER").unwrap(),
            env::var("TEST_PG_PASSWORD").unwrap(),
            env::var("TEST_PG_HOST").unwrap(),
            env::var("TEST_PG_PORT").unwrap(),
            env::var("TEST_PG_DB").unwrap(),
        );

        let url = Url::parse(&conn_string).unwrap();
        let pool = crate::pool::postgres(url).unwrap();

        let conn = pool.check_out().await.unwrap();
        let result_set = conn.query_raw("SHOW search_path", &[]).await.unwrap();
        let row = result_set.first().unwrap();

        assert_eq!(Some("\"musti-test\""), row[0].as_str());
    }
}
