use std::sync::Arc;

use crate::logger::log_error_and_exit;
use schema_connector::ConnectorError;
use schema_core::{
    DatasourceUrls, ExtensionTypeConfig,
    json_rpc::types::{DatasourceParam, UrlContainer},
};
use structopt::StructOpt;
use tokio_util::sync::CancellationToken;

#[derive(Debug, StructOpt)]
pub(crate) struct Cli {
    #[structopt(subcommand)]
    command: CliCommand,
}

impl Cli {
    pub(crate) async fn run(
        self,
        datasource_urls: DatasourceUrls,
        shutdown_token: CancellationToken,
        extensions: Arc<ExtensionTypeConfig>,
    ) {
        match self.run_inner(datasource_urls, shutdown_token, extensions).await {
            Ok(msg) => {
                tracing::info!("{}", msg);
            }
            Err(error) => log_error_and_exit(error),
        }
    }

    pub(crate) async fn run_inner(
        self,
        datasource_urls: DatasourceUrls,
        shutdown_token: CancellationToken,
        extensions: Arc<ExtensionTypeConfig>,
    ) -> Result<String, ConnectorError> {
        let mut api = schema_core::schema_api(None, datasource_urls.clone(), None, extensions)?;

        let url = datasource_urls
            .url
            .ok_or_else(|| ConnectorError::from_msg("No URL defined in the configured datasource".to_owned()))?;

        let work = async {
            match self.command {
                CliCommand::CreateDatabase => api
                    .create_database(schema_core::json_rpc::types::CreateDatabaseParams {
                        datasource: DatasourceParam::ConnectionString(UrlContainer { url }),
                    })
                    .await
                    .map(|schema_core::json_rpc::types::CreateDatabaseResult { database_name }| {
                        format!("Database '{database_name}' was successfully created.")
                    }),
                CliCommand::CanConnectToDatabase => api
                    .ensure_connection_validity(schema_core::json_rpc::types::EnsureConnectionValidityParams {
                        datasource: DatasourceParam::ConnectionString(UrlContainer { url }),
                    })
                    .await
                    .map(|_| "Connection successful".to_owned()),
                CliCommand::DropDatabase => api
                    .drop_database(url)
                    .await
                    .map(|_| "The database was successfully dropped.".to_owned()),
            }
        };

        let result = tokio::select! {
            result = work => result,
            _ = shutdown_token.cancelled() => Err(ConnectorError::from_msg("Operation was cancelled".to_owned())),
        };

        api.dispose().await?;

        result
    }
}

#[derive(Debug, StructOpt)]
#[allow(clippy::enum_variant_names)] // disagee
enum CliCommand {
    /// Create an empty database defined in the configuration string.
    CreateDatabase,
    /// Does the database connection string work?
    CanConnectToDatabase,
    /// Drop the database.
    DropDatabase,
}
