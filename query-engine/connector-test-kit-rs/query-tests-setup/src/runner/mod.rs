mod json_adapter;

pub use json_adapter::*;
use serde::Deserialize;

use crate::{
    executor_process_request, ConnectorTag, ConnectorVersion, QueryResult, TestError, TestLogCapture, TestResult,
    ENGINE_PROTOCOL,
};
use colored::Colorize;
use query_core::{
    protocol::EngineProtocol,
    schema::{self, QuerySchemaRef},
    QueryExecutor, TransactionOptions, TxId,
};
use query_engine_metrics::MetricRegistry;
use request_handlers::{
    BatchTransactionOption, ConnectorMode, GraphqlBody, JsonBatchQuery, JsonBody, JsonSingleQuery, MultiQuery,
    RequestBody, RequestHandler,
};
use serde_json::json;
use std::{
    env,
    sync::{atomic::AtomicUsize, Arc},
};

pub type TxResult = Result<(), user_facing_errors::Error>;

pub(crate) type Executor = Box<dyn QueryExecutor + Send + Sync>;

#[derive(Deserialize, Debug)]
struct Empty {}

#[derive(Deserialize, Debug)]
#[serde(untagged)]
enum TransactionEndResponse {
    Error(user_facing_errors::Error),
    Ok(Empty),
}

impl From<TransactionEndResponse> for TxResult {
    fn from(value: TransactionEndResponse) -> Self {
        match value {
            TransactionEndResponse::Ok(_) => Ok(()),
            TransactionEndResponse::Error(error) => Err(error),
        }
    }
}

pub enum RunnerExecutor {
    // Builtin is a runner that uses the query engine in-process, issuing queries against a
    // `core::InterpretingExecutor` that uses the particular connector under test in the test suite.
    Builtin(Executor),

    // External is a runner that uses an external process that responds to queries piped to its STDIN
    // in JsonRPC format. In particular this is used to test the query engine against a node process
    // running a library engine configured to use a javascript driver adapter to connect to a database.
    //
    // In this struct variant, usize represents the index of the schema used for the test suite to
    // execute queries against. When the suite starts, a message with the schema and the id is sent to
    // the external process, which will create a new instance of the library engine configured to
    // access that schema.
    //
    // Everytime a query is sent to the external process, it's provided the id of the schema, so the
    // process knows how to associate the query to the instance of the library engine that will dispatch
    // it.
    External(usize),
}

impl RunnerExecutor {
    async fn new_external(url: &str, schema: &str) -> TestResult<RunnerExecutor> {
        static COUNTER: AtomicUsize = AtomicUsize::new(0);
        let id = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        executor_process_request(
            "initializeSchema",
            json!({ "schema": schema, "schemaId": id, "url": url }),
        )
        .await?;

        Ok(RunnerExecutor::External(id))
    }
}

/// Direct engine runner.
pub struct Runner {
    executor: RunnerExecutor,
    query_schema: QuerySchemaRef,
    version: ConnectorVersion,
    connector_tag: ConnectorTag,
    connection_url: String,
    current_tx_id: Option<TxId>,
    metrics: MetricRegistry,
    protocol: EngineProtocol,
    log_capture: TestLogCapture,
}

impl Runner {
    pub(crate) fn schema_id(&self) -> Option<usize> {
        match self.executor {
            RunnerExecutor::Builtin(_) => None,
            RunnerExecutor::External(schema_id) => Some(schema_id),
        }
    }

    pub fn prisma_dml(&self) -> &str {
        self.query_schema.internal_data_model.schema.db.source()
    }

    pub async fn load(
        datamodel: String,
        db_schemas: &[&str],
        connector_version: ConnectorVersion,
        connector_tag: ConnectorTag,
        metrics: MetricRegistry,
        log_capture: TestLogCapture,
    ) -> TestResult<Self> {
        qe_setup::setup(&datamodel, db_schemas).await?;

        let protocol = EngineProtocol::from(&ENGINE_PROTOCOL.to_string());
        let schema = psl::parse_schema(&datamodel).unwrap();
        let data_source = schema.configuration.datasources.first().unwrap();
        let url = data_source.load_url(|key| env::var(key).ok()).unwrap();

        let executor = match crate::CONFIG.external_test_executor() {
            Some(_) => RunnerExecutor::new_external(&url, &datamodel).await?,
            None => RunnerExecutor::Builtin(
                request_handlers::load_executor(
                    ConnectorMode::Rust,
                    data_source,
                    schema.configuration.preview_features(),
                    &url,
                )
                .await?,
            ),
        };
        let query_schema: QuerySchemaRef = Arc::new(schema::build(Arc::new(schema), true));

        Ok(Self {
            version: connector_version,
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

        let executor = match &self.executor {
            RunnerExecutor::Builtin(e) => e,
            RunnerExecutor::External(schema_id) => match JsonRequest::from_graphql(&query, self.query_schema()) {
                Ok(json_query) => {
                    let response_str: String =
                    executor_process_request("query", json!({ "query": json_query, "schemaId": schema_id, "txId": self.current_tx_id.as_ref().map(ToString::to_string) })).await?;
                    let mut response: QueryResult = serde_json::from_str(&response_str).unwrap();
                    response.detag();
                    return Ok(response);
                }
                // Conversion from graphql to JSON might fail, and in that case we should consider the error
                // (a Handler error) as an error response.
                Err(TestError::RequestHandlerError(err)) => {
                    let gql_err = request_handlers::GQLError::from_handler_error(err);
                    let gql_res = request_handlers::GQLResponse::from(gql_err);
                    let prisma_res = request_handlers::PrismaResponse::Single(gql_res);
                    let mut response = QueryResult::from(prisma_res);
                    response.detag();
                    return Ok(response);
                }
                Err(err) => return Err(err),
            },
        };

        tracing::debug!("Querying: {}", query.clone().green());

        let handler = RequestHandler::new(&**executor, &self.query_schema, self.protocol);

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

        let executor = match &self.executor {
            RunnerExecutor::Builtin(e) => e,
            RunnerExecutor::External(_) => {
                let response_str: String = executor_process_request(
                    "query",
                    json!({ "query": query, "txId": self.current_tx_id.as_ref().map(ToString::to_string) }),
                )
                .await?;
                let response: QueryResult = serde_json::from_str(&response_str).unwrap();
                return Ok(response);
            }
        };

        let handler = RequestHandler::new(&**executor, &self.query_schema, EngineProtocol::Json);

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

        self.connector_tag.raw_execute(&query, &self.connection_url).await?;

        Ok(())
    }

    pub async fn batch_json(
        &self,
        queries: Vec<String>,
        transaction: bool,
        isolation_level: Option<String>,
    ) -> TestResult<crate::QueryResult> {
        let executor = match &self.executor {
            RunnerExecutor::External(_) => todo!(),
            RunnerExecutor::Builtin(e) => e,
        };

        let handler = RequestHandler::new(&**executor, &self.query_schema, self.protocol);
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
        let executor = match &self.executor {
            RunnerExecutor::External(schema_id) => {
                // Translate the GraphQL query to JSON
                let batch = queries
                    .into_iter()
                    .map(|query| JsonRequest::from_graphql(&query, self.query_schema()))
                    .collect::<TestResult<Vec<_>>>()
                    .unwrap();
                let transaction = match transaction {
                    true => Some(BatchTransactionOption { isolation_level }),
                    false => None,
                };
                let json_query = JsonBody::Batch(JsonBatchQuery { batch, transaction });
                let response_str: String = executor_process_request(
                        "query", 
                        json!({ "query": json_query, "schemaId": schema_id, "txId": self.current_tx_id.as_ref().map(ToString::to_string) })
                    ).await?;

                let mut response: QueryResult = serde_json::from_str(&response_str).unwrap();
                response.detag();
                return Ok(response);
            }
            RunnerExecutor::Builtin(e) => e,
        };

        let handler = RequestHandler::new(&**executor, &self.query_schema, self.protocol);
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
        new_tx_id: Option<TxId>,
    ) -> TestResult<TxId> {
        let tx_opts = TransactionOptions::new(max_acquisition_millis, valid_for_millis, isolation_level, new_tx_id);
        match &self.executor {
            RunnerExecutor::Builtin(executor) => {
                let id = executor
                    .start_tx(self.query_schema.clone(), self.protocol, tx_opts)
                    .await?;
                Ok(id)
            }
            RunnerExecutor::External(schema_id) => {
                #[derive(Deserialize, Debug)]
                #[serde(untagged)]
                enum StartTransactionResponse {
                    Ok { id: String },
                    Error(user_facing_errors::Error),
                }
                let response: StartTransactionResponse =
                    executor_process_request("startTx", json!({ "schemaId": schema_id, "options": tx_opts })).await?;

                match response {
                    StartTransactionResponse::Ok { id } => Ok(id.into()),
                    StartTransactionResponse::Error(err) => {
                        Err(crate::TestError::InteractiveTransactionError(err.message().into()))
                    }
                }
            }
        }
    }

    pub async fn commit_tx(&self, tx_id: TxId) -> TestResult<TxResult> {
        match &self.executor {
            RunnerExecutor::Builtin(executor) => {
                let res = executor.commit_tx(tx_id).await;

                if let Err(error) = res {
                    Ok(Err(error.into()))
                } else {
                    Ok(Ok(()))
                }
            }
            RunnerExecutor::External(schema_id) => {
                let response: TransactionEndResponse =
                    executor_process_request("commitTx", json!({ "schemaId": schema_id, "txId": tx_id.to_string() }))
                        .await?;

                Ok(response.into())
            }
        }
    }

    pub async fn rollback_tx(&self, tx_id: TxId) -> TestResult<TxResult> {
        match &self.executor {
            RunnerExecutor::Builtin(executor) => {
                let res = executor.rollback_tx(tx_id).await;

                if let Err(error) = res {
                    Ok(Err(error.into()))
                } else {
                    Ok(Ok(()))
                }
            }
            RunnerExecutor::External(schema_id) => {
                let response: TransactionEndResponse = executor_process_request(
                    "rollbackTx",
                    json!({ "schemaId": schema_id, "txId": tx_id.to_string() }),
                )
                .await?;

                Ok(response.into())
            }
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
        let mut logs = self.log_capture.get_logs().await;
        match &self.executor {
            RunnerExecutor::Builtin(_) => logs,
            RunnerExecutor::External(schema_id) => {
                let mut external_logs: Vec<String> =
                    executor_process_request("getLogs", json!({ "schemaId": schema_id }))
                        .await
                        .unwrap();
                logs.append(&mut external_logs);
                logs
            }
        }
    }

    pub fn connector_version(&self) -> &ConnectorVersion {
        &self.version
    }

    pub fn protocol(&self) -> EngineProtocol {
        self.protocol
    }

    pub fn is_external_executor(&self) -> bool {
        matches!(self.executor, RunnerExecutor::External(_))
    }
}
