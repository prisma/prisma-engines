mod json_adapter;

pub use json_adapter::*;

use crate::{ConnectorTag, ConnectorVersion, QueryResult, TestLogCapture, TestResult, ENGINE_PROTOCOL};
use colored::Colorize;
use quaint::{prelude::Queryable, single::Quaint};
use query_core::{
    protocol::EngineProtocol,
    schema::{self, QuerySchemaRef},
    QueryExecutor, TransactionOptions, TxId,
};
use query_engine_metrics::MetricRegistry;
use request_handlers::{
    load_executor, BatchTransactionOption, GraphqlBody, JsonBatchQuery, JsonBody, JsonSingleQuery, MultiQuery,
    RequestBody, RequestHandler,
};
use std::{env, sync::Arc};

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
    protocol: EngineProtocol,
    log_capture: TestLogCapture,
}

impl Runner {
    pub fn prisma_dml(&self) -> &str {
        self.query_schema.internal_data_model.schema.db.source()
    }

    pub async fn load(
        datamodel: String,
        db_schemas: &[&str],
        connector_tag: ConnectorTag,
        metrics: MetricRegistry,
        log_capture: TestLogCapture,
    ) -> TestResult<Self> {
        qe_setup::setup(&datamodel, db_schemas).await?;

        let protocol = EngineProtocol::from(&ENGINE_PROTOCOL.to_string());
        let schema = psl::parse_schema(datamodel).unwrap();
        let data_source = schema.configuration.datasources.first().unwrap();
        let url = data_source.load_url(|key| env::var(key).ok()).unwrap();
        let executor = load_executor(data_source, schema.configuration.preview_features(), &url).await?;
        let query_schema: QuerySchemaRef = Arc::new(schema::build(Arc::new(schema), true));

        Ok(Self {
            executor,
            query_schema,
            connector_tag,
            connection_url: url,
            current_tx_id: None,
            metrics,
            protocol,
            log_capture,
        })
    }

    pub async fn query<T>(&self, query: T) -> TestResult<QueryResult>
    where
        T: Into<String>,
    {
        let query = query.into();

        tracing::debug!("Querying: {}", query.clone().green());

        let handler = RequestHandler::new(&*self.executor, &self.query_schema, self.protocol);

        let request_body = match self.protocol {
            EngineProtocol::Json => {
                // Translate the GraphQL query to JSON
                let json_query = JsonRequest::from_graphql(&query, self.query_schema()).unwrap();
                println!("{}", serde_json::to_string_pretty(&json_query).unwrap().green());

                RequestBody::Json(JsonBody::Single(json_query))
            }
            EngineProtocol::Graphql => {
                println!("{}", query.bright_green());

                RequestBody::Graphql(GraphqlBody::Single(query.into()))
            }
        };

        let response = handler.handle(request_body, self.current_tx_id.clone(), None).await;

        let result: QueryResult = match self.protocol {
            EngineProtocol::Json => JsonResponse::from_graphql(response).into(),
            EngineProtocol::Graphql => response.into(),
        };

        if result.failed() {
            tracing::debug!("Response: {}", result.to_string().red());
        } else {
            tracing::debug!("Response: {}", result.to_string().green());
        }

        Ok(result)
    }

    pub async fn query_json<T>(&self, query: T) -> TestResult<QueryResult>
    where
        T: Into<String>,
    {
        let query = query.into();

        tracing::debug!("Querying: {}", query.clone().green());

        println!("{}", query.bright_green());

        let handler = RequestHandler::new(&*self.executor, &self.query_schema, EngineProtocol::Json);

        let serialized_query: JsonSingleQuery = serde_json::from_str(&query).unwrap();
        let request_body = RequestBody::Json(JsonBody::Single(serialized_query));

        let result: QueryResult = handler
            .handle(request_body, self.current_tx_id.clone(), None)
            .await
            .into();

        if result.failed() {
            tracing::debug!("Response: {}", result.to_string().red());
        } else {
            tracing::debug!("Response: {}", result.to_string().green());
        }

        Ok(result)
    }

    pub async fn raw_execute<T>(&self, query: T) -> TestResult<()>
    where
        T: Into<String>,
    {
        let query = query.into();
        tracing::debug!("Raw execute: {}", query.clone().green());

        if matches!(self.connector_tag, ConnectorTag::MongoDb(_)) {
            panic!("raw_execute is not supported for MongoDB yet");
        }

        let conn = Quaint::new(&self.connection_url).await?;
        conn.raw_cmd(&query).await?;

        Ok(())
    }

    pub async fn batch_json(
        &self,
        queries: Vec<String>,
        transaction: bool,
        isolation_level: Option<String>,
    ) -> TestResult<crate::QueryResult> {
        let handler = RequestHandler::new(&*self.executor, &self.query_schema, self.protocol);
        let body = RequestBody::Json(JsonBody::Batch(JsonBatchQuery {
            batch: queries
                .into_iter()
                .map(|q| serde_json::from_str::<JsonSingleQuery>(&q).unwrap())
                .collect(),
            transaction: transaction.then_some(BatchTransactionOption { isolation_level }),
        }));

        let res = handler.handle(body, self.current_tx_id.clone(), None).await;

        Ok(res.into())
    }

    pub async fn batch(
        &self,
        queries: Vec<String>,
        transaction: bool,
        isolation_level: Option<String>,
    ) -> TestResult<crate::QueryResult> {
        let handler = RequestHandler::new(&*self.executor, &self.query_schema, self.protocol);
        let body = match self.protocol {
            EngineProtocol::Json => {
                // Translate the GraphQL query to JSON
                let batch = queries
                    .into_iter()
                    .map(|query| JsonRequest::from_graphql(&query, self.query_schema()))
                    .collect::<TestResult<Vec<_>>>()
                    .unwrap();
                let transaction_opts = match transaction {
                    true => Some(BatchTransactionOption { isolation_level }),
                    false => None,
                };

                println!("{}", serde_json::to_string_pretty(&batch).unwrap().green());

                RequestBody::Json(JsonBody::Batch(JsonBatchQuery {
                    batch,
                    transaction: transaction_opts,
                }))
            }
            EngineProtocol::Graphql => RequestBody::Graphql(GraphqlBody::Multi(MultiQuery::new(
                queries.into_iter().map(Into::into).collect(),
                transaction,
                isolation_level,
            ))),
        };

        let res = handler.handle(body, self.current_tx_id.clone(), None).await;

        match self.protocol {
            EngineProtocol::Json => Ok(JsonResponse::from_graphql(res).into()),
            EngineProtocol::Graphql => Ok(res.into()),
        }
    }

    pub async fn start_tx(
        &self,
        max_acquisition_millis: u64,
        valid_for_millis: u64,
        isolation_level: Option<String>,
    ) -> TestResult<TxId> {
        let tx_opts = TransactionOptions::new(max_acquisition_millis, valid_for_millis, isolation_level);

        let id = self
            .executor
            .start_tx(self.query_schema.clone(), self.protocol, tx_opts)
            .await?;
        Ok(id)
    }

    pub async fn commit_tx(&self, tx_id: TxId) -> TestResult<TxResult> {
        let res = self.executor.commit_tx(tx_id).await;

        if let Err(error) = res {
            Ok(Err(error.into()))
        } else {
            Ok(Ok(()))
        }
    }

    pub async fn rollback_tx(&self, tx_id: TxId) -> TestResult<TxResult> {
        let res = self.executor.rollback_tx(tx_id).await;

        if let Err(error) = res {
            Ok(Err(error.into()))
        } else {
            Ok(Ok(()))
        }
    }

    pub fn connector(&self) -> &crate::ConnectorTag {
        &self.connector_tag
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

    pub async fn get_logs(&mut self) -> Vec<String> {
        self.log_capture.get_logs().await
    }

    pub fn connector_version(&self) -> ConnectorVersion {
        ConnectorVersion::from(self.connector())
    }

    pub fn protocol(&self) -> EngineProtocol {
        self.protocol
    }
}
