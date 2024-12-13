use std::sync::Arc;

use async_trait::async_trait;
use postgres_types::{BorrowToSql, Type};
use tokio_postgres::{Client, Error, RowStream, Statement};

/// Types that can be dispatched to the database as a query and carry the necessary type
/// information about its parameters and columns to interpret the results.
#[async_trait]
pub trait PreparedQuery: Send {
    fn param_types(&self) -> impl ExactSizeIterator<Item = &Type>;
    fn column_names(&self) -> impl ExactSizeIterator<Item = &str>;
    fn column_types(&self) -> impl ExactSizeIterator<Item = &Type>;

    async fn dispatch<Args>(&self, client: &Client, args: Args) -> Result<RowStream, Error>
    where
        Args: IntoIterator + Send,
        Args::Item: BorrowToSql,
        Args::IntoIter: ExactSizeIterator + Send;
}

#[async_trait]
impl PreparedQuery for Statement {
    fn param_types(&self) -> impl ExactSizeIterator<Item = &Type> {
        self.params().iter()
    }

    fn column_names(&self) -> impl ExactSizeIterator<Item = &str> {
        self.columns().iter().map(|c| c.name())
    }

    fn column_types(&self) -> impl ExactSizeIterator<Item = &Type> {
        self.columns().iter().map(|c| c.type_())
    }

    async fn dispatch<Args>(&self, client: &Client, args: Args) -> Result<RowStream, Error>
    where
        Args: IntoIterator + Send,
        Args::Item: BorrowToSql,
        Args::IntoIter: ExactSizeIterator + Send,
    {
        client.query_raw(self, args).await
    }
}

/// A query combined with the relevant type information about its parameters and columns.
#[derive(Debug)]
pub struct TypedQuery<'a> {
    pub(super) sql: &'a str,
    pub(super) metadata: Arc<QueryMetadata>,
}

impl<'a> TypedQuery<'a> {
    /// Create a new typed query from an SQL string and metadata.
    pub fn from_sql_and_metadata(sql: &'a str, metadata: impl Into<Arc<QueryMetadata>>) -> Self {
        Self {
            sql,
            metadata: metadata.into(),
        }
    }
}

#[async_trait]
impl<'a> PreparedQuery for TypedQuery<'a> {
    fn param_types(&self) -> impl ExactSizeIterator<Item = &Type> {
        self.metadata.param_types.iter()
    }

    fn column_names(&self) -> impl ExactSizeIterator<Item = &str> {
        self.metadata.column_names.iter().map(|s| s.as_str())
    }

    fn column_types(&self) -> impl ExactSizeIterator<Item = &Type> {
        self.metadata.column_types.iter()
    }

    async fn dispatch<Args>(&self, client: &Client, args: Args) -> Result<RowStream, Error>
    where
        Args: IntoIterator + Send,
        Args::Item: BorrowToSql,
        Args::IntoIter: ExactSizeIterator + Send,
    {
        let typed_args = args.into_iter().zip(self.metadata.param_types.iter().cloned());
        client.query_typed_raw(self.sql, typed_args).await
    }
}

#[derive(Debug)]
pub struct QueryMetadata {
    param_types: Vec<Type>,
    column_names: Vec<String>,
    column_types: Vec<Type>,
}

impl From<&Statement> for QueryMetadata {
    fn from(statement: &Statement) -> Self {
        Self {
            param_types: statement.params().to_vec(),
            column_names: statement.columns().iter().map(|c| c.name().to_owned()).collect(),
            column_types: statement.columns().iter().map(|c| c.type_().clone()).collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::future::Future;

    pub(crate) use crate::connector::postgres::url::PostgresNativeUrl;
    use crate::{
        connector::{MakeTlsConnectorManager, PostgresFlavour},
        tests::test_api::postgres::CONN_STR,
    };
    use url::Url;

    #[tokio::test]
    async fn typed_query_matches_statement_and_dispatches() {
        run_with_client(|client| async move {
            let query = "SELECT $1";
            let stmt = client.prepare_typed(query, &[Type::INT4]).await.unwrap();
            let typed = TypedQuery::from_sql_and_metadata(query, QueryMetadata::from(&stmt));

            assert_eq!(typed.param_types().cloned().collect::<Vec<_>>(), stmt.params());
            assert_eq!(
                typed.column_names().collect::<Vec<_>>(),
                stmt.columns().iter().map(|c| c.name()).collect::<Vec<_>>()
            );
            assert_eq!(
                typed.column_types().collect::<Vec<_>>(),
                stmt.columns().iter().map(|c| c.type_()).collect::<Vec<_>>()
            );

            let result = typed.dispatch(&client, &[&1i32]).await;
            assert!(result.is_ok(), "{:?}", result.err());
        })
        .await;
    }

    #[tokio::test]
    async fn statement_trait_methods_match_statement_and_dispatch() {
        run_with_client(|client| async move {
            let query = "SELECT $1";
            let stmt = client.prepare_typed(query, &[Type::INT4]).await.unwrap();

            assert_eq!(stmt.param_types().cloned().collect::<Vec<_>>(), stmt.params());
            assert_eq!(
                stmt.column_names().collect::<Vec<_>>(),
                stmt.columns().iter().map(|c| c.name()).collect::<Vec<_>>()
            );
            assert_eq!(
                stmt.column_types().collect::<Vec<_>>(),
                stmt.columns().iter().map(|c| c.type_()).collect::<Vec<_>>()
            );

            let result = stmt.dispatch(&client, &[&1i32]).await;
            assert!(result.is_ok(), "{:?}", result.err());
        })
        .await;
    }

    async fn run_with_client<Func, Fut>(test: Func)
    where
        Func: FnOnce(Client) -> Fut,
        Fut: Future<Output = ()>,
    {
        let url = Url::parse(&CONN_STR).unwrap();
        let mut pg_url = PostgresNativeUrl::new(url).unwrap();
        pg_url.set_flavour(PostgresFlavour::Postgres);

        let tls_manager = MakeTlsConnectorManager::new(pg_url.clone());
        let tls = tls_manager.get_connector().await.unwrap();

        let (client, conn) = pg_url.to_config().connect(tls).await.unwrap();

        let set = tokio::task::LocalSet::new();
        set.spawn_local(conn);
        set.run_until(test(client)).await
    }
}
