#![allow(clippy::declare_interior_mutable_const)]

//! What the executor module DOES:
//! - Defining an overarching executor trait, to be used on consumers of the core crate.
//! - Defining executor implementations that combine the different core modules into a coherent
//!   string of actions to execute a given query document.
//!
//! What the executor module DOES NOT DO:
//! - Define low level execution of queries. This is considered an implementation detail of the modules used by the executors.
mod execute_operation;
mod interpreting_executor;
mod loader;
mod pipeline;
mod request_context;

pub use execute_operation::*;
pub use loader::*;
pub(crate) use request_context::*;
use serde::Deserialize;

use crate::{
    protocol::EngineProtocol, query_document::Operation, response_ir::ResponseData, schema::QuerySchemaRef,
    BatchDocumentTransaction, TxId,
};
use async_trait::async_trait;
use connector::Connector;
use tracing::Dispatch;

#[async_trait]
pub trait QueryExecutor: TransactionManager {
    /// Executes a single operation and returns its result.
    /// Implementers must honor the passed transaction ID and execute the operation on the transaction identified
    /// by `tx_id`. If `None`, implementers are free to choose how to execute the query.
    async fn execute(
        &self,
        tx_id: Option<TxId>,
        operation: Operation,
        query_schema: QuerySchemaRef,
        trace_id: Option<String>,
        engine_protocol: EngineProtocol,
    ) -> crate::Result<ResponseData>;

    /// Executes a collection of operations as either a fanout of individual operations (non-transactional), or in series (transactional).
    ///
    /// Implementers must honor the passed transaction ID and execute the operation on the transaction identified
    /// by `tx_id`. If `None`, implementers are free to choose how to execute the query.
    ///
    /// Note that `transactional` is the legacy marker for transactional batches. It must be supported until the stabilization of ITXs.
    async fn execute_all(
        &self,
        tx_id: Option<TxId>,
        operations: Vec<Operation>,
        transaction: Option<BatchDocumentTransaction>,
        query_schema: QuerySchemaRef,
        trace_id: Option<String>,
        engine_protocol: EngineProtocol,
    ) -> crate::Result<Vec<crate::Result<ResponseData>>>;

    fn primary_connector(&self) -> &(dyn Connector + Send + Sync);
}

#[derive(Debug, Deserialize)]
pub struct TransactionOptions {
    /// Maximum wait time for tx acquisition in milliseconds.
    #[serde(rename(deserialize = "max_wait"))]
    pub max_acquisition_millis: u64,

    /// Time in milliseconds after which the transaction rolls back automatically.
    #[serde(rename(deserialize = "timeout"))]
    pub valid_for_millis: u64,

    /// Isolation level to use for the transaction.
    pub isolation_level: Option<String>,

    /// An optional pre-defined transaction id. Some value might be provided in case we want to generate
    /// a new id at the beginning of the transaction
    #[serde(skip_deserializing)]
    pub new_tx_id: Option<TxId>,
}

impl TransactionOptions {
    pub fn new(max_acquisition_millis: u64, valid_for_millis: u64, isolation_level: Option<String>) -> Self {
        Self {
            max_acquisition_millis,
            valid_for_millis,
            isolation_level,
            new_tx_id: None,
        }
    }

    /// Generates a new transaction id before the transaction is started and returns a modified version
    /// of self with the new predefined_id set.
    pub fn with_new_transaction_id(&mut self) -> TxId {
        let tx_id: TxId = Default::default();
        self.new_tx_id = Some(tx_id.clone());
        tx_id
    }
}
#[async_trait]
pub trait TransactionManager {
    /// Starts a new transaction.
    /// Returns ID of newly opened transaction.
    /// Expected to throw an error if no transaction could be opened for `opts.max_acquisition_millis` milliseconds.
    /// The new transaction must only live for `opts.valid_for_millis` milliseconds before it automatically rolls back.
    /// This rollback mechanism is an implementation detail of the trait implementer.
    async fn start_tx(
        &self,
        query_schema: QuerySchemaRef,
        engine_protocol: EngineProtocol,
        opts: TransactionOptions,
    ) -> crate::Result<TxId>;

    /// Commits a transaction.
    async fn commit_tx(&self, tx_id: TxId) -> crate::Result<()>;

    /// Rolls back a transaction.
    async fn rollback_tx(&self, tx_id: TxId) -> crate::Result<()>;
}

// With the node-api when a future is spawned in a new thread `tokio:spawn` it will not
// use the current dispatcher and its logs will not be captured anymore. We can use this
// method to get the current dispatcher and combine it with `with_subscriber`
// let dispatcher = get_current_dispatcher();
// tokio::spawn(async {
//      my_async_ops.await
// }.with_subscriber(dispatcher));
//
//
// Finally, this can be replaced with with_current_collector
// https://github.com/tokio-rs/tracing/blob/master/tracing-futures/src/lib.rs#L234
// once this is in a release

pub fn get_current_dispatcher() -> Dispatch {
    tracing::dispatcher::get_default(|current| current.clone())
}
