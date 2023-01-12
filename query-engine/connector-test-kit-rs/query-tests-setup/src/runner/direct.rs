use crate::{ConnectorTag, RunnerInterface, TestResult, TxResult};
use colored::Colorize;
use query_core::{executor, schema::QuerySchemaRef, schema_builder, QueryExecutor, TxId};
use query_engine_metrics::MetricRegistry;
use request_handlers::{GraphQlBody, GraphQlHandler, MultiQuery};
use std::{env, sync::Arc};
use tokio::sync::OnceCell;

use quaint::{prelude::Queryable, single::Quaint};

pub(crate) type Executor = Box<dyn QueryExecutor + Send + Sync>;

/// Direct engine runner.
pub struct DirectRunner {
    executor: Executor,
    query_schema: QuerySchemaRef,
    connector_tag: ConnectorTag,
    quaint: OnceCell<Quaint>,
    connection_url: String,
    current_tx_id: Option<TxId>,
    metrics: MetricRegistry,
}

impl DirectRunner {
    // Avoid fetching a new database connection unless needed.
    async fn quaint(&self) -> &Quaint {
        if matches!(&self.connector_tag, ConnectorTag::MongoDb(_)) {
            unimplemented!("quaint cannot be instantiated when the active connector is MongoDb");
        }

        self.quaint
            .get_or_try_init(|| Quaint::new(&self.connection_url))
            .await
            .unwrap()
    }
}

#[async_trait::async_trait]
impl RunnerInterface for DirectRunner {
    async fn load(datamodel: String, connector_tag: ConnectorTag, metrics: MetricRegistry) -> TestResult<Self> {
        let schema = psl::parse_schema(datamodel).unwrap();
        let data_source = schema.configuration.datasources.first().unwrap();
        let url = data_source.load_url(|key| env::var(key).ok()).unwrap();
        let (db_name, executor) = executor::load(data_source, schema.configuration.preview_features(), &url).await?;
        let internal_data_model = prisma_models::convert(Arc::new(schema), db_name);

        let query_schema: QuerySchemaRef = Arc::new(schema_builder::build(internal_data_model, true));

        Ok(Self {
            executor,
            query_schema,
            connector_tag,
            quaint: OnceCell::default(),
            connection_url: url,
            current_tx_id: None,
            metrics,
        })
    }

    async fn query(&self, query: String) -> TestResult<crate::QueryResult> {
        println!("{}", query.bright_green());

        let handler = GraphQlHandler::new(&*self.executor, &self.query_schema);
        let query = GraphQlBody::Single(query.into());

        Ok(handler.handle(query, self.current_tx_id.clone(), None).await.into())
    }

    async fn raw_execute(&self, query: String) -> TestResult<()> {
        let quaint = Quaint::new(&self.connection_url).await.unwrap();

        quaint.raw_cmd(&query).await.map_err(crate::TestError::RawExecute)
    }

    async fn batch(
        &self,
        queries: Vec<String>,
        transaction: bool,
        isolation_level: Option<String>,
    ) -> TestResult<crate::QueryResult> {
        let handler = GraphQlHandler::new(&*self.executor, &self.query_schema);
        let query = GraphQlBody::Multi(MultiQuery::new(
            queries.into_iter().map(Into::into).collect(),
            transaction,
            isolation_level,
        ));

        Ok(handler.handle(query, self.current_tx_id.clone(), None).await.into())
    }

    async fn start_tx(
        &self,
        max_acquisition_millis: u64,
        valid_for_millis: u64,
        isolation_level: Option<String>,
    ) -> TestResult<TxId> {
        let id = self
            .executor
            .start_tx(
                self.query_schema.clone(),
                max_acquisition_millis,
                valid_for_millis,
                isolation_level,
            )
            .await?;
        Ok(id)
    }

    async fn commit_tx(&self, tx_id: TxId) -> TestResult<TxResult> {
        let res = self.executor.commit_tx(tx_id).await;

        if let Err(error) = res {
            return Ok(Err(error.into()));
        } else {
            Ok(Ok(()))
        }
    }

    async fn rollback_tx(&self, tx_id: TxId) -> TestResult<TxResult> {
        let res = self.executor.rollback_tx(tx_id).await;

        if let Err(error) = res {
            return Ok(Err(error.into()));
        } else {
            Ok(Ok(()))
        }
    }

    fn connector(&self) -> &crate::ConnectorTag {
        &self.connector_tag
    }

    fn set_active_tx(&mut self, tx_id: query_core::TxId) {
        self.current_tx_id = Some(tx_id);
    }

    fn clear_active_tx(&mut self) {
        self.current_tx_id = None;
    }

    fn get_metrics(&self) -> MetricRegistry {
        self.metrics.clone()
    }

    fn query_schema(&self) -> &QuerySchemaRef {
        &self.query_schema
    }

    async fn schema_name(&self) -> &str {
        self.quaint().await.connection_info().schema_name()
    }
}
