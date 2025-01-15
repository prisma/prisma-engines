use crate::SqlFlavour;
use quaint::connector::ExternalConnector;

pub struct SqliteFlavour {
    connector: Arc<dyn ExternalConnector>,
}

impl SqlFlavour for SqliteFlavour {
    // Note: this bypasses the `with_connection` and `quaint_err` helpers from `sql-schema-connector/src/flavour/sqlite`.
    async fn version(&mut self) -> BoxFuture<'_, ConnectorResult<Option<String>>> {
        tracing::debug!(query_type = "version");
        self.connector.version().await
    }

    async fn ensure_connection_validity(&mut self) -> ConnectorResult<()> {
        // TODO: verify that a connection can be established
        Ok(())
    }
}
