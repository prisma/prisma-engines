use crate::{ConnectorTag, JsonRequest, JsonResponse, RunnerInterface, TestResult, TxResult};
use colored::Colorize;
use query_core::{
    executor, protocol::EngineProtocol, schema::QuerySchemaRef, schema_builder, QueryExecutor, TransactionOptions, TxId,
};
use query_engine_metrics::MetricRegistry;
use request_handlers::{
    BatchTransactionOption, GraphqlBody, JsonBatchQuery, JsonBody, JsonSingleQuery, MultiQuery, RequestBody,
    RequestHandler,
};
use std::{env, sync::Arc};

use quaint::{prelude::Queryable, single::Quaint};

pub(crate) type Executor = Box<dyn QueryExecutor + Send + Sync>;

/// Direct engine runner.
pub struct DirectRunner {
    executor: Executor,
    query_schema: QuerySchemaRef,
    connector_tag: ConnectorTag,
    connection_url: String,
    current_tx_id: Option<TxId>,
    metrics: MetricRegistry,
}

#[async_trait::async_trait]
impl RunnerInterface for DirectRunner {
    async fn load(datamodel: String, connector_tag: ConnectorTag, metrics: MetricRegistry) -> TestResult<Self> {
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

    async fn query_graphql(&self, query: String, protocol: &EngineProtocol) -> TestResult<crate::QueryResult> {
        let handler = RequestHandler::new(&*self.executor, &self.query_schema, *protocol);

        let request_body = match protocol {
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

        match protocol {
            EngineProtocol::Json => Ok(JsonResponse::from_graphql(response).into()),
            EngineProtocol::Graphql => Ok(response.into()),
        }
    }

    async fn query_json(&self, query: String) -> TestResult<crate::QueryResult> {
        println!("{}", query.bright_green());

        let handler = RequestHandler::new(&*self.executor, &self.query_schema, EngineProtocol::Json);

        let serialized_query: JsonSingleQuery = serde_json::from_str(&query).unwrap();
        let request_body = RequestBody::Json(JsonBody::Single(serialized_query));

        Ok(handler
            .handle(request_body, self.current_tx_id.clone(), None)
            .await
            .into())
    }

    async fn raw_execute(&self, query: String) -> TestResult<()> {
        if matches!(self.connector_tag, ConnectorTag::MongoDb(_)) {
            panic!("raw_execute is not supported for MongoDB yet");
        }

        let conn = Quaint::new(&self.connection_url).await?;
        conn.raw_cmd(&query).await?;

        Ok(())
    }

    async fn batch(
        &self,
        queries: Vec<String>,
        transaction: bool,
        isolation_level: Option<String>,
        engine_protocol: EngineProtocol,
    ) -> TestResult<crate::QueryResult> {
        let handler = RequestHandler::new(&*self.executor, &self.query_schema, engine_protocol);
        let body = match engine_protocol {
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

        match engine_protocol {
            EngineProtocol::Json => Ok(JsonResponse::from_graphql(res).into()),
            EngineProtocol::Graphql => Ok(res.into()),
        }
    }

    async fn start_tx(
        &self,
        max_acquisition_millis: u64,
        valid_for_millis: u64,
        isolation_level: Option<String>,
        engine_protocol: EngineProtocol,
    ) -> TestResult<TxId> {
        let tx_opts = TransactionOptions::new(max_acquisition_millis, valid_for_millis, isolation_level);

        let id = self
            .executor
            .start_tx(self.query_schema.clone(), engine_protocol, tx_opts)
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
}
