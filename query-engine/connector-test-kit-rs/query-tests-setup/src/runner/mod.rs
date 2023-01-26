use colored::*;
use quaint::{prelude::Queryable, single::Quaint};
use query_core::{executor, schema::QuerySchemaRef, schema_builder, QueryExecutor, TransactionOptions, TxId};
use query_engine_metrics::MetricRegistry;
use request_handlers::{GraphQlBody, GraphQlHandler, MultiQuery};
use std::{env, sync::Arc};

use crate::{ConnectorTag, ConnectorVersion, TestResult};

pub type TxResult = Result<(), user_facing_errors::Error>;

pub(crate) type Executor = Box<dyn QueryExecutor + Send + Sync>;

/// Direct engine runner.
pub struct Runner {
    executor: Executor,
    query_schema: QuerySchemaRef,
    connector_tag: ConnectorTag,
    connection_url: String,
    current_tx_id: Option<TxId>,
    metrics: MetricRegistry,
}

impl Runner {
    pub(crate) async fn load(
        datamodel: String,
        connector_tag: ConnectorTag,
        metrics: MetricRegistry,
    ) -> TestResult<Self> {
        let schema = psl::parse_schema(datamodel).unwrap();
        let data_source = schema.configuration.datasources.first().unwrap();
        let url = data_source.load_url(|key| env::var(key).ok()).unwrap();
        let executor = executor::load(data_source, schema.configuration.preview_features(), &url).await?;
        let internal_data_model = prisma_models::convert(Arc::new(schema));

        let query_schema: QuerySchemaRef = Arc::new(schema_builder::build(internal_data_model, true));

        Ok(Self {
            executor,
            query_schema,
            connector_tag,
            connection_url: url,
            current_tx_id: None,
            metrics,
        })
    }

    pub async fn query<T: Into<String>>(&self, query: T) -> TestResult<crate::QueryResult> {
        let query = query.into();
        tracing::debug!("Querying: {}", query.green());
        println!("{}", query.bright_green());

        let handler = GraphQlHandler::new(&*self.executor, &self.query_schema);
        let query = GraphQlBody::Single(query.into());

        let response: crate::QueryResult = handler.handle(query, self.current_tx_id.clone(), None).await.into();

        if response.failed() {
            tracing::debug!("Response: {}", response.to_string().red());
        } else {
            tracing::debug!("Response: {}", response.to_string().green());
        }

        Ok(response)
    }

    pub async fn raw_execute<T: Into<String>>(&self, query: T) -> TestResult<()> {
        let query = query.into();
        if matches!(self.connector_tag, ConnectorTag::MongoDb(_)) {
            panic!("raw_execute is not supported for MongoDB yet");
        }

        tracing::debug!("Raw execute: {}", query.green());

        let conn = Quaint::new(&self.connection_url).await?;
        conn.raw_cmd(&query).await?;

        Ok(())
    }

    pub async fn batch(
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

    pub async fn start_tx(
        &self,
        max_acquisition_millis: u64,
        valid_for_millis: u64,
        isolation_level: Option<String>,
    ) -> TestResult<TxId> {
        let tx_opts = TransactionOptions::new(max_acquisition_millis, valid_for_millis, isolation_level);

        let id = self.executor.start_tx(self.query_schema.clone(), tx_opts).await?;
        Ok(id)
    }

    pub async fn commit_tx(&self, tx_id: TxId) -> TestResult<TxResult> {
        let res = self.executor.commit_tx(tx_id).await;

        if let Err(error) = res {
            return Ok(Err(error.into()));
        } else {
            Ok(Ok(()))
        }
    }

    pub async fn rollback_tx(&self, tx_id: TxId) -> TestResult<TxResult> {
        let res = self.executor.rollback_tx(tx_id).await;

        if let Err(error) = res {
            return Ok(Err(error.into()));
        } else {
            Ok(Ok(()))
        }
    }

    pub fn connector(&self) -> &ConnectorTag {
        &self.connector_tag
    }

    pub fn connector_version(&self) -> ConnectorVersion {
        ConnectorVersion::from(self.connector())
    }

    pub fn set_active_tx(&mut self, tx_id: query_core::TxId) {
        self.current_tx_id = Some(tx_id);
    }

    pub fn clear_active_tx(&mut self) {
        self.current_tx_id = None;
    }

    pub fn get_metrics(&self) -> MetricRegistry {
        self.metrics.clone()
    }

    pub fn query_schema(&self) -> &QuerySchemaRef {
        &self.query_schema
    }
}
