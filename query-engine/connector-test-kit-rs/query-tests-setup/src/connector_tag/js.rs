mod node_process;

use super::*;
use node_process::*;
use query_core::{
    executor::TransactionManager, protocol::EngineProtocol, response_ir::ResponseData, schema::QuerySchemaRef,
    BatchDocumentTransaction, Connector, Operation, QueryExecutor, TransactionOptions, TxId,
};
use serde::Deserialize;
use serde_json::json;
use std::{collections::HashMap, sync::atomic::AtomicU64};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

#[derive(Debug, Default, Clone, PartialEq)]
pub struct NodeDrivers;

impl ConnectorTagInterface for NodeDrivers {
    fn raw_execute<'a>(&'a self, query: &'a str, connection_url: &'a str) -> BoxFuture<'a, Result<(), TestError>> {
        Box::pin(async move {
            NODE_PROCESS
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
        &NODE_PROCESS.config.datamodel_provider
    }

    fn datamodel_renderer(&self) -> Box<dyn DatamodelRenderer> {
        todo!()
    }

    fn capabilities(&self) -> ConnectorCapabilities {
        todo!()
    }
}

#[async_trait::async_trait]
impl TransactionManager for ExecutorProcess {
    async fn start_tx(
        &self,
        query_schema: QuerySchemaRef,
        engine_protocol: EngineProtocol,
        opts: TransactionOptions,
    ) -> query_core::Result<TxId> {
        todo!()
    }

    async fn commit_tx(&self, tx_id: TxId) -> Result<(), query_core::CoreError> {
        todo!()
    }

    async fn rollback_tx(&self, tx_id: TxId) -> Result<(), query_core::CoreError> {
        todo!()
    }
}

#[async_trait::async_trait]
impl QueryExecutor for ExecutorProcess {
    async fn execute(
        &self,
        tx_id: Option<TxId>,
        operation: Operation,
        query_schema: QuerySchemaRef,
        trace_id: Option<String>,
        engine_protocol: EngineProtocol,
    ) -> query_core::Result<ResponseData> {
        todo!()
    }

    async fn execute_all(
        &self,
        tx_id: Option<TxId>,
        operations: Vec<Operation>,
        transaction: Option<BatchDocumentTransaction>,
        query_schema: QuerySchemaRef,
        trace_id: Option<String>,
        engine_protocol: EngineProtocol,
    ) -> query_core::Result<Vec<query_core::Result<ResponseData>>> {
        todo!()
    }

    fn primary_connector(&self) -> &(dyn Connector + Send + Sync) {
        todo!()
    }
}
