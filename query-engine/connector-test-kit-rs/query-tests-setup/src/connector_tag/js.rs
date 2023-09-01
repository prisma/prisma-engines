mod node_process;

use super::*;
use node_process::*;
use query_core::{
    executor::TransactionManager, protocol::EngineProtocol, schema::QuerySchemaRef, TransactionOptions, TxId,
};
use serde::{de::DeserializeOwned, Deserialize};
use serde_json::json;
use std::{collections::HashMap, sync::atomic::AtomicU64};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

pub(crate) async fn executor_process_request<T: DeserializeOwned>(
    method: &str,
    params: serde_json::Value,
) -> Result<T, Box<dyn std::error::Error + Send + Sync>> {
    NODE_PROCESS.0.request(method, params).await
}

#[derive(Debug, Default, Clone, PartialEq)]
pub struct NodeDrivers;

impl ConnectorTagInterface for NodeDrivers {
    fn raw_execute<'a>(&'a self, query: &'a str, connection_url: &'a str) -> BoxFuture<'a, Result<(), TestError>> {
        Box::pin(async move {
            NODE_PROCESS
                .0
                .request::<()>(
                    "rawExecute",
                    json!({
                        "query": query,
                        "connection_url": connection_url,
                    }),
                )
                .await
                .unwrap();
            Ok(())
        })
    }

    fn datamodel_provider(&self) -> &str {
        &NODE_PROCESS.1.datamodel_provider
    }

    fn datamodel_renderer(&self) -> Box<dyn DatamodelRenderer> {
        match self.datamodel_provider() {
            "postgresql" => todo!(),
            provider => todo!("provider: {provider}"),
        }
    }

    fn capabilities(&self) -> ConnectorCapabilities {
        todo!()
    }
}

#[async_trait::async_trait]
impl TransactionManager for ExecutorProcess {
    async fn start_tx(
        &self,
        _query_schema: QuerySchemaRef,
        _engine_protocol: EngineProtocol,
        _opts: TransactionOptions,
    ) -> query_core::Result<TxId> {
        let txid: String = NODE_PROCESS.0.request("startTx", json!(null)).await.map_err(|err| {
            query_core::CoreError::ConnectorError(query_core::ConnectorError::from_kind(
                query_core::ConnectorErrorKind::RawDatabaseError {
                    code: String::from("0"),
                    message: err.to_string(),
                },
            ))
        })?;

        Ok(txid.into())
    }

    async fn commit_tx(&self, tx_id: TxId) -> Result<(), query_core::CoreError> {
        NODE_PROCESS
            .0
            .request("commitTx", json!({ "txId": tx_id.to_string() }))
            .await
            .map_err(|err| {
                query_core::CoreError::ConnectorError(query_core::ConnectorError::from_kind(
                    query_core::ConnectorErrorKind::RawDatabaseError {
                        code: String::from("0"),
                        message: err.to_string(),
                    },
                ))
            })?;

        Ok(())
    }

    async fn rollback_tx(&self, tx_id: TxId) -> Result<(), query_core::CoreError> {
        NODE_PROCESS
            .0
            .request("rollbackTx", json!({ "txId": tx_id.to_string() }))
            .await
            .map_err(|err| {
                query_core::CoreError::ConnectorError(query_core::ConnectorError::from_kind(
                    query_core::ConnectorErrorKind::RawDatabaseError {
                        code: String::from("0"),
                        message: err.to_string(),
                    },
                ))
            })?;

        Ok(())
    }
}

// #[async_trait::async_trait]
// impl QueryExecutor for ExecutorProcess {
//     async fn execute(
//         &self,
//         tx_id: Option<TxId>,
//         operation: Operation,
//         query_schema: QuerySchemaRef,
//         trace_id: Option<String>,
//         engine_protocol: EngineProtocol,
//     ) -> query_core::Result<ResponseData> {
//         todo!()
//     }

//     async fn execute_all(
//         &self,
//         tx_id: Option<TxId>,
//         operations: Vec<Operation>,
//         transaction: Option<BatchDocumentTransaction>,
//         query_schema: QuerySchemaRef,
//         trace_id: Option<String>,
//         engine_protocol: EngineProtocol,
//     ) -> query_core::Result<Vec<query_core::Result<ResponseData>>> {
//         todo!()
//     }

//     fn primary_connector(&self) -> &(dyn Connector + Send + Sync) {
//         registered_js_connector(NodeDrivers.datamodel_provider())
//     }
// }

// #[async_trait]
// impl Connector for NodeDrivers {
//     async fn get_connection(&self) -> crate::Result<Box<dyn Connection + Send + Sync>> {
//         todo!();
//     }

//     fn name(&self) -> &'static str {
//         self.datamodel_provider()
//     }

//     fn should_retry_on_transient_error(&self) -> bool {
//         false
//     }
// }
