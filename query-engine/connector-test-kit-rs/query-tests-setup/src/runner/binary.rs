use crate::{ConnectorTag, JsonRequest, RunnerInterface, TestError, TestResult, TxResult};
use colored::Colorize;
use query_core::protocol::EngineProtocol;
use query_core::{schema::QuerySchemaRef, TxId};
use query_engine::opt::PrismaOpt;
use query_engine::server::routes;
use query_engine::state::{setup, State};
use query_engine_metrics::MetricRegistry;
use request_handlers::{
    BatchTransactionOption, GQLBatchResponse, GQLError, GQLResponse, GraphQlBody, JsonBatchQuery, JsonBody,
    JsonSingleQuery, MultiQuery, PrismaResponse, RequestBody,
};

use hyper::{Body, Method, Request, Response};
use quaint::{prelude::Queryable, single::Quaint};
use std::env;

pub struct BinaryRunner {
    connector_tag: ConnectorTag,
    current_tx_id: Option<TxId>,
    state: State,
    connection_url: String,
}

impl BinaryRunner {
    async fn query(&self, body: Vec<u8>) -> TestResult<crate::QueryResult> {
        let mut builder = Request::builder().method(Method::POST);

        if self.current_tx_id.is_some() {
            let tx_id: String = self.current_tx_id.clone().unwrap().to_string();
            builder = builder.header("X-transaction-id", tx_id);
        }

        let req = builder.body(Body::from(body)).unwrap();

        let resp = routes(self.state.clone(), req).await.unwrap();

        let json_resp: serde_json::Value = response_to_json(resp).await;
        let gql_response = json_to_gql_response(&json_resp);

        Ok(PrismaResponse::Single(gql_response).into())
    }
}

#[async_trait::async_trait]
impl RunnerInterface for BinaryRunner {
    async fn load(datamodel: String, connector_tag: ConnectorTag, metrics: MetricRegistry) -> TestResult<Self> {
        let opts = PrismaOpt::from_list(&[
            "binary",
            "--enable-raw-queries",
            "--enable-telemetry-in-response",
            "--datamodel",
            &datamodel,
        ]);
        let state = setup(&opts, false, Some(metrics)).await.unwrap();

        let configuration = opts.configuration(true).unwrap();
        let data_source = configuration.datasources.first().unwrap();
        let connection_url = data_source.load_url(|key| env::var(key).ok()).unwrap();

        Ok(BinaryRunner {
            state,
            connector_tag,
            connection_url,
            current_tx_id: None,
        })
    }

    async fn query_graphql(&self, query: String, protocol: &EngineProtocol) -> TestResult<crate::QueryResult> {
        let body = match protocol {
            EngineProtocol::Json => {
                // Translate the GraphQL query to JSON
                let json_query = JsonRequest::from_graphql(&query, self.query_schema()).unwrap();
                println!("{}", serde_json::to_string_pretty(&json_query).unwrap().bright_green());

                let query = JsonBody::Single(json_query);

                serde_json::to_vec(&query).unwrap()
            }
            EngineProtocol::Graphql => {
                println!("{}", query.bright_green());

                let query = GraphQlBody::Single(query.into());

                serde_json::to_vec(&query).unwrap()
            }
        };

        self.query(body).await
    }

    async fn query_json(&self, query: String) -> TestResult<crate::QueryResult> {
        println!("{}", query.bright_green());

        let query: JsonSingleQuery = serde_json::from_str(&query).unwrap();
        let query = JsonBody::Single(query);
        let body = serde_json::to_vec(&query).unwrap();

        self.query(body).await
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
        let body = match engine_protocol {
            EngineProtocol::Json => {
                let batch = queries
                    .into_iter()
                    .map(|query| JsonRequest::from_graphql(&query, self.query_schema()))
                    .collect::<TestResult<Vec<_>>>()
                    .unwrap();
                let transaction_opts = match transaction {
                    true => Some(BatchTransactionOption { isolation_level }),
                    false => None,
                };

                RequestBody::Json(JsonBody::Batch(JsonBatchQuery {
                    batch,
                    transaction: transaction_opts,
                }))
            }
            EngineProtocol::Graphql => RequestBody::Graphql(GraphQlBody::Multi(MultiQuery::new(
                queries.into_iter().map(Into::into).collect(),
                transaction,
                isolation_level,
            ))),
        };

        let body = body.try_as_bytes().unwrap();

        let mut builder = Request::builder().method(Method::POST);

        // Garren: basically if there is a current_tx_id we run it as a transaction
        // I don't fully understand how ITX works and I need to do this to pass the tests
        if self.current_tx_id.is_some() {
            let tx_id: String = self.current_tx_id.as_ref().unwrap().clone().to_string();
            builder = builder.header("X-transaction-id", tx_id);
        }

        let req = builder.body(Body::from(body)).unwrap();

        let resp = routes(self.state.clone(), req).await.unwrap();
        let json_resp: serde_json::Value = response_to_json(resp).await;

        let mut batch_response = GQLBatchResponse::default();

        if let Some(batch) = json_resp.get("batchResult") {
            let results = batch.as_array().unwrap();
            let responses: Vec<GQLResponse> = results.iter().map(json_to_gql_response).collect();
            batch_response.insert_responses(responses);
        }

        if let Some(error_val) = json_resp.get("errors") {
            let errors = error_val.as_array().unwrap();

            errors.iter().for_each(|err| {
                let gql_error: GQLError = serde_json::from_value(err.clone()).unwrap();
                batch_response.insert_error(gql_error);
            })
        }

        Ok(PrismaResponse::Multi(batch_response).into())
    }

    async fn start_tx(
        &self,
        max_acquisition_millis: u64,
        valid_for_millis: u64,
        isolation_level: Option<String>,
        _engine_protocol: EngineProtocol,
    ) -> TestResult<TxId> {
        let body = serde_json::json!({
            "max_wait": max_acquisition_millis,
            "timeout": valid_for_millis,
            "isolation_level": isolation_level,
        });

        let body_bytes = serde_json::to_vec(&body).unwrap();

        let req = Request::builder()
            .uri("/transaction/start")
            .method(Method::POST)
            .body(Body::from(body_bytes))
            .unwrap();

        let resp = routes(self.state.clone(), req).await.unwrap();
        let json_resp = response_to_json(resp).await;
        let json_obj = json_resp.as_object().unwrap();

        match json_obj.get("error_code") {
            Some(_) => Err(TestError::InteractiveTransactionError(
                serde_json::to_string(json_obj).unwrap(),
            )),
            None => Ok(json_obj.get("id").unwrap().as_str().unwrap().into()),
        }
    }

    async fn commit_tx(&self, tx_id: TxId) -> TestResult<TxResult> {
        let uri = format!("/transaction/{tx_id}/commit");

        let req = Request::builder()
            .uri(uri.as_str())
            .method(Method::POST)
            .body(Body::from(r#"{}"#))
            .unwrap();

        let resp = routes(self.state.clone(), req).await;
        let resp = resp.unwrap();

        let result = response_to_json(resp).await;
        let error: Result<user_facing_errors::Error, _> = serde_json::from_value(result);

        if let Ok(user_error) = error {
            Ok(Err(user_error))
        } else {
            Ok(Ok(()))
        }
    }

    async fn rollback_tx(&self, tx_id: TxId) -> TestResult<TxResult> {
        let uri = format!("/transaction/{tx_id}/rollback");

        let req = Request::builder()
            .uri(uri.as_str())
            .method(Method::POST)
            .body(Body::from(r#"{}"#))
            .unwrap();

        let resp = routes(self.state.clone(), req).await.unwrap();
        let result = response_to_json(resp).await;

        let error: Result<user_facing_errors::Error, _> = serde_json::from_value(result);

        if let Ok(user_error) = error {
            Ok(Err(user_error))
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
        self.state.get_metrics()
    }

    fn query_schema(&self) -> &QuerySchemaRef {
        self.state.query_schema()
    }
}

async fn response_to_json(resp: Response<Body>) -> serde_json::Value {
    let body_start = resp.into_body();
    let full_body = hyper::body::to_bytes(body_start).await.unwrap();

    serde_json::from_slice(full_body.as_ref()).unwrap()
}

fn json_to_gql_response(json_resp: &serde_json::Value) -> GQLResponse {
    let mut gql_response = match json_resp.get("data") {
        Some(data_val) => {
            let obj = data_val.as_object().unwrap();

            let mut gql_response = GQLResponse::with_capacity(obj.keys().count());

            obj.iter().for_each(|(k, v)| {
                gql_response.insert_data(k.to_string(), query_core::response_ir::Item::Json(v.clone()));
            });
            gql_response
        }
        None => GQLResponse::with_capacity(0),
    };

    if let Some(error_val) = json_resp.get("errors") {
        let errors = error_val.as_array().unwrap();

        errors.iter().for_each(|err| {
            let gql_error: GQLError = serde_json::from_value(err.clone()).unwrap();
            gql_response.insert_error(gql_error);
        })
    }

    gql_response
}
