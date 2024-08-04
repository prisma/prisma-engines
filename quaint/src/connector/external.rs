use async_trait::async_trait;

use super::{SqlFamily, TransactionCapable};

#[derive(Debug, Clone)]
pub struct ExternalConnectionInfo {
    pub sql_family: SqlFamily,
    pub schema_name: String,
    pub max_bind_values: Option<usize>,
}

impl ExternalConnectionInfo {
    pub fn new(sql_family: SqlFamily, schema_name: String, max_bind_values: Option<usize>) -> Self {
        ExternalConnectionInfo {
            sql_family,
            schema_name,
            max_bind_values,
        }
    }
}

#[async_trait]
pub trait ExternalConnector: TransactionCapable {
    async fn get_connection_info(&self) -> crate::Result<ExternalConnectionInfo>;
}
