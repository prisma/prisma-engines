use crate::error::quaint_error_to_connector_error;
use datamodel::{walkers::walk_scalar_fields, Datamodel};
use migration_connector::ConnectorResult;
use quaint::{
    prelude::{ConnectionInfo, Queryable, SqlFamily},
    single::Quaint,
};

#[derive(Debug, Clone)]
pub struct DatabaseInfo {
    connection_info: ConnectionInfo,
    pub database_version: Option<String>,
}

impl DatabaseInfo {
    pub(crate) async fn new(connection: &Quaint, connection_info: ConnectionInfo) -> ConnectorResult<Self> {
        let database_version = connection
            .version()
            .await
            .map_err(|err| quaint_error_to_connector_error(err, &connection_info))?;

        Ok(DatabaseInfo {
            connection_info,
            database_version,
        })
    }

    pub(crate) fn is_mysql_5_6(&self) -> bool {
        self.connection_info.sql_family() == SqlFamily::Mysql
            && self
                .database_version
                .as_ref()
                .map(|version| version.contains("5.6"))
                .unwrap_or(false)
    }

    pub(crate) fn is_mariadb(&self) -> bool {
        self.connection_info.sql_family() == SqlFamily::Mysql
            && self
                .database_version
                .as_ref()
                .map(|version| version.contains("MariaDB"))
                .unwrap_or(false)
    }

    pub(crate) fn sql_family(&self) -> SqlFamily {
        self.connection_info.sql_family()
    }

    pub(crate) fn connection_info(&self) -> &ConnectionInfo {
        &self.connection_info
    }

    pub(crate) fn check_database_version_compatibility(
        &self,
        datamodel: &Datamodel,
    ) -> Option<user_facing_errors::common::DatabaseVersionIncompatibility> {
        let mut errors = Vec::new();

        if self.is_mysql_5_6() {
            check_datamodel_for_mysql_5_6(datamodel, &mut errors)
        }

        if errors.is_empty() {
            return None;
        }

        let mut errors_string = String::with_capacity(errors.iter().map(|err| err.len() + 3).sum());

        for error in &errors {
            errors_string.push_str("- ");
            errors_string.push_str(error);
            errors_string.push_str("\n");
        }

        Some(user_facing_errors::common::DatabaseVersionIncompatibility {
            errors: errors_string,
            database_version: self.database_version.as_ref().unwrap().clone(),
        })
    }
}

fn check_datamodel_for_mysql_5_6(datamodel: &Datamodel, errors: &mut Vec<String>) {
    walk_scalar_fields(datamodel).for_each(|field| {
        if field.field_type().is_json() {
            errors.push(format!(
                "The `Json` data type used in {}.{} is not supported on MySQL 5.6.",
                field.model().name(),
                field.name()
            ))
        }
    });
}
