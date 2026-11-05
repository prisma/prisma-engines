mod destructive_change_checker;
mod renderer;
mod schema_calculator;
mod schema_differ;

#[cfg(feature = "kingbase-mysql-native")]
mod connector;

use super::{MysqlDialect, SqlDialect};
use crate::{sql_destructive_change_checker::DestructiveChangeCheckerFlavour, sql_renderer::SqlRenderer};
use destructive_change_checker::KingbaseMysqlDestructiveChangeCheckerFlavour;
use psl::{PreviewFeatures, ValidatedSchema};
use schema_calculator::KingbaseMysqlSchemaCalculatorFlavour;
use schema_connector::{BoxFuture, ConnectorResult};
use schema_differ::KingbaseMysqlSchemaDifferFlavour;

pub(crate) use renderer::KingbaseMysqlRenderer;

/// C010 cannot inline a GIN expression or a descending unique index in CREATE
/// TABLE. Keep rendering, planning and calculated constraint ownership aligned.
fn index_is_created_separately(index: sql_schema_describer::walkers::IndexWalker<'_>) -> bool {
    index.index_type() == sql_schema_describer::IndexType::Fulltext
        || (index.is_unique()
            && index
                .columns()
                .any(|column| column.sort_order() == Some(sql_schema_describer::SQLSortOrder::Desc)))
}

#[cfg(feature = "kingbase-mysql-native")]
pub(crate) use connector::KingbaseMysqlConnector;

#[derive(Debug, Default)]
pub(crate) struct KingbaseMysqlDialect {
    mysql: MysqlDialect,
}

impl SqlDialect for KingbaseMysqlDialect {
    fn renderer(&self) -> Box<dyn SqlRenderer> {
        Box::new(KingbaseMysqlRenderer)
    }

    fn schema_differ(&self) -> Box<dyn crate::sql_schema_differ::SqlSchemaDifferFlavour> {
        Box::new(KingbaseMysqlSchemaDifferFlavour)
    }

    fn schema_calculator(&self) -> Box<dyn crate::sql_schema_calculator::SqlSchemaCalculatorFlavour> {
        Box::new(KingbaseMysqlSchemaCalculatorFlavour)
    }

    fn destructive_change_checker(&self) -> Box<dyn DestructiveChangeCheckerFlavour> {
        Box::new(KingbaseMysqlDestructiveChangeCheckerFlavour)
    }

    fn check_schema_features(&self, schema: &ValidatedSchema) -> ConnectorResult<()> {
        self.mysql.check_schema_features(schema)
    }

    fn datamodel_connector(&self) -> &'static dyn psl::datamodel_connector::Connector {
        psl::builtin_connectors::KINGBASE_MYSQL
    }

    #[cfg(any(
        feature = "mssql-native",
        feature = "mysql-native",
        feature = "kingbase-mysql-native",
        feature = "postgresql-native",
        feature = "sqlite-native"
    ))]
    fn connect_to_shadow_db(
        &self,
        url: String,
        preview_features: PreviewFeatures,
    ) -> BoxFuture<'_, ConnectorResult<Box<dyn super::SqlConnector>>> {
        #[cfg(feature = "kingbase-mysql-native")]
        {
            let params = schema_connector::ConnectorParams::new(url, preview_features, None);
            Box::pin(async move {
                Ok(Box::new(KingbaseMysqlConnector::new_with_params(params)?) as Box<dyn super::SqlConnector>)
            })
        }

        #[cfg(not(feature = "kingbase-mysql-native"))]
        {
            let _ = (url, preview_features);
            Box::pin(async {
                Err(schema_connector::ConnectorError::from_msg(
                    "Kingbase MySQL shadow database support is not implemented yet.".to_owned(),
                ))
            })
        }
    }

    #[cfg(not(any(
        feature = "mssql-native",
        feature = "mysql-native",
        feature = "kingbase-mysql-native",
        feature = "postgresql-native",
        feature = "sqlite-native"
    )))]
    fn connect_to_shadow_db(
        &self,
        _factory: std::sync::Arc<dyn quaint::connector::ExternalConnectorFactory>,
    ) -> BoxFuture<'_, ConnectorResult<Box<dyn super::SqlConnector>>> {
        Box::pin(async {
            Err(schema_connector::ConnectorError::from_msg(
                "Kingbase MySQL shadow database support is not implemented yet.".to_owned(),
            ))
        })
    }
}
