//! What the executor module DOES:
//! - Defining an overarching executor trait, to be used on consumers of the core crate.
//! - Defining executor implementations that combine the different core modules into a coherent
//!   string of actions to execute a given query document.
//!
//! What the executor module DOES NOT DO:
//! - Define low level execution of queries. This is considered an implementation detail of the modules used by the executors.
mod interactive_tx;
mod interpreting_executor;
mod loader;
mod pipeline;

pub use interactive_tx::*;
pub use loader::*;

use crate::{query_document::Operation, response_ir::ResponseData, schema::QuerySchemaRef};
use async_trait::async_trait;
use connector::Connector;

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
        transactional: bool,
        query_schema: QuerySchemaRef,
    ) -> crate::Result<Vec<crate::Result<ResponseData>>>;

    fn primary_connector(&self) -> &(dyn Connector + Send + Sync);
}

#[async_trait]
pub trait TransactionManager {
    /// Starts a new transaction.
    /// Returns ID of newly opened transaction.
    /// Expected to throw an error if no transaction could be opened for `max_acquisition_millis` milliseconds.
    /// The new transaction must only live for `valid_for_millis` milliseconds before it automatically rolls back.
    /// This rollback mechanism is an implementation detail of the trait implementer.
    async fn start_tx(&self, max_acquisition_millis: u64, valid_for_millis: u64) -> crate::Result<TxId>;

    /// Commits a transaction.
    async fn commit_tx(&self, tx_id: TxId) -> crate::Result<()>;

    /// Rolls back a transaction.
    async fn rollback_tx(&self, tx_id: TxId) -> crate::Result<()>;
}
