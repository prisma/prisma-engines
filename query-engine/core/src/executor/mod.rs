//! What the executor module DOES:
//! - Defining an overarching executor trait, to be used on consumers of the core crate.
//! - Defining executor implementations that combine the different core modules into a coherent
//!   string of actions to execute a given query document.
//!
//! What the executor module DOES NOT DO:
//! - Define low level execution of queries. This is considered an implementation detail of the modules used by the executors.
mod interpreting_executor;
mod pipeline;

pub use interpreting_executor::*;

use crate::{query_document::Operation, response_ir::ResponseData, schema::QuerySchemaRef};
use async_trait::async_trait;
use connector::Connector;

#[async_trait]
pub trait QueryExecutor {
    /// Executes a single operation and returns its result.
    async fn execute(&self, operation: Operation, query_schema: QuerySchemaRef) -> crate::Result<ResponseData>;

    // Executes a batch of operations as either a fanout of individual operations (non-transactional), or in series (transactional).
    async fn execute_batch(
        &self,
        operations: Vec<Operation>,
        transactional: bool,
        query_schema: QuerySchemaRef,
    ) -> crate::Result<Vec<crate::Result<ResponseData>>>;

    fn primary_connector(&self) -> &dyn Connector;
}
