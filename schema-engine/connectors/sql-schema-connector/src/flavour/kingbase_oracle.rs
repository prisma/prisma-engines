mod destructive_change_checker;
mod renderer;
mod schema_calculator;
mod schema_differ;

#[cfg(feature = "kingbase-oracle-native")]
mod connector;

use super::SqlDialect;
use crate::{sql_destructive_change_checker::DestructiveChangeCheckerFlavour, sql_renderer::SqlRenderer};
use destructive_change_checker::KingbaseOracleDestructiveChangeCheckerFlavour;
#[cfg(any(
    feature = "mssql-native",
    feature = "mysql-native",
    feature = "kingbase-mysql-native",
    feature = "kingbase-oracle-native",
    feature = "postgresql-native",
    feature = "sqlite-native"
))]
use psl::PreviewFeatures;
use schema_calculator::KingbaseOracleSchemaCalculatorFlavour;
#[cfg(not(feature = "kingbase-oracle-native"))]
use schema_connector::ConnectorError;
use schema_connector::{BoxFuture, ConnectorResult};
use schema_differ::KingbaseOracleSchemaDifferFlavour;

pub(crate) use renderer::KingbaseOracleRenderer;

#[cfg(feature = "kingbase-oracle-native")]
pub(crate) use connector::KingbaseOracleConnector;

/// SQL dialect for KingbaseES in Oracle compatibility mode.
#[derive(Debug, Default)]
pub(crate) struct KingbaseOracleDialect;

impl SqlDialect for KingbaseOracleDialect {
    fn renderer(&self) -> Box<dyn SqlRenderer> {
        Box::new(KingbaseOracleRenderer)
    }

    fn schema_differ(&self) -> Box<dyn crate::sql_schema_differ::SqlSchemaDifferFlavour> {
        Box::new(KingbaseOracleSchemaDifferFlavour)
    }

    fn schema_calculator(&self) -> Box<dyn crate::sql_schema_calculator::SqlSchemaCalculatorFlavour> {
        Box::new(KingbaseOracleSchemaCalculatorFlavour)
    }

    fn destructive_change_checker(&self) -> Box<dyn DestructiveChangeCheckerFlavour> {
        Box::new(KingbaseOracleDestructiveChangeCheckerFlavour)
    }

    fn datamodel_connector(&self) -> &'static dyn psl::datamodel_connector::Connector {
        psl::builtin_connectors::KINGBASE_ORACLE
    }

    fn default_namespace(&self) -> Option<&str> {
        Some("public")
    }

    #[cfg(any(
        feature = "mssql-native",
        feature = "mysql-native",
        feature = "kingbase-mysql-native",
        feature = "kingbase-oracle-native",
        feature = "postgresql-native",
        feature = "sqlite-native"
    ))]
    fn connect_to_shadow_db(
        &self,
        url: String,
        preview_features: PreviewFeatures,
    ) -> BoxFuture<'_, ConnectorResult<Box<dyn super::SqlConnector>>> {
        #[cfg(feature = "kingbase-oracle-native")]
        {
            let params = schema_connector::ConnectorParams::new(url, preview_features, None);
            Box::pin(async move {
                Ok(Box::new(KingbaseOracleConnector::new_with_params(params)?) as Box<dyn super::SqlConnector>)
            })
        }

        #[cfg(not(feature = "kingbase-oracle-native"))]
        {
            let _ = (url, preview_features);
            Box::pin(async {
                Err(ConnectorError::from_msg(
                    "Kingbase Oracle shadow database support is not implemented yet.".to_owned(),
                ))
            })
        }
    }

    #[cfg(not(any(
        feature = "mssql-native",
        feature = "mysql-native",
        feature = "kingbase-mysql-native",
        feature = "kingbase-oracle-native",
        feature = "postgresql-native",
        feature = "sqlite-native"
    )))]
    fn connect_to_shadow_db(
        &self,
        _factory: std::sync::Arc<dyn quaint::connector::ExternalConnectorFactory>,
    ) -> BoxFuture<'_, ConnectorResult<Box<dyn super::SqlConnector>>> {
        Box::pin(async {
            Err(ConnectorError::from_msg(
                "Kingbase Oracle shadow database support is not implemented yet.".to_owned(),
            ))
        })
    }
}
