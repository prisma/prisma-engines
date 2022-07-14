//! All the quaint-wrangling for the mysql connector should happen here.

use enumflags2::BitFlags;
use migration_connector::{ConnectorError, ConnectorResult};
use quaint::{
    connector::{
        self,
        mysql_async::{self as my, prelude::Query},
        MysqlUrl,
    },
    prelude::{ConnectionInfo, Queryable},
};
use sql_schema_describer::{DescriberErrorKind, SqlSchema, SqlSchemaDescriberBackend};
use user_facing_errors::{
    introspection_engine::DatabaseSchemaInconsistent,
    migration_engine::{ApplyMigrationError, DirectDdlNotAllowed, ForeignKeyCreationNotAllowed},
    KnownError,
};

pub(super) struct Connection(connector::Mysql);

impl Connection {
    pub(super) async fn new(url: url::Url) -> ConnectorResult<Connection> {
        let url = MysqlUrl::new(url).map_err(|err| {
            ConnectorError::user_facing(user_facing_errors::common::InvalidConnectionString {
                details: err.to_string(),
            })
        })?;
        Ok(Connection(
            connector::Mysql::new(url.clone()).await.map_err(quaint_err(&url))?,
        ))
    }

    pub(super) async fn describe_schema(
        &mut self,
        params: &super::Params,
    ) -> ConnectorResult<SqlSchema> {
        let mut schema = sql_schema_describer::mysql::SqlSchemaDescriber::new(&self.0)
            .describe(params.url.dbname())
            .await
            .map_err(|err| match err.into_kind() {
                DescriberErrorKind::QuaintError(err) => quaint_err(&params.url)(err),
                e @ DescriberErrorKind::CrossSchemaReference { .. } => {
                    let err = DatabaseSchemaInconsistent {
                        explanation: e.to_string(),
                    };
                    ConnectorError::user_facing(err)
                }
            })?;

        crate::flavour::normalize_sql_schema(&mut schema, params.connector_params.preview_features);

        Ok(schema)
    }

    pub(super) async fn raw_cmd(&mut self, sql: &str, url: &MysqlUrl) -> ConnectorResult<()> {
        tracing::debug!(query_type = "raw_cmd", sql);
        self.0.raw_cmd(sql).await.map_err(quaint_err(url))
    }

    pub(super) async fn version(&mut self, url: &MysqlUrl) -> ConnectorResult<Option<String>> {
        tracing::debug!(query_type = "version");
        self.0.version().await.map_err(quaint_err(url))
    }

    pub(super) async fn query(
        &mut self,
        query: quaint::ast::Query<'_>,
        url: &MysqlUrl,
    ) -> ConnectorResult<quaint::prelude::ResultSet> {
        use quaint::visitor::Visitor;
        let (sql, params) = quaint::visitor::Mysql::build(query).unwrap();
        self.query_raw(&sql, &params, url).await
    }

    pub(super) async fn query_raw(
        &mut self,
        sql: &str,
        params: &[quaint::prelude::Value<'_>],
        url: &MysqlUrl,
    ) -> ConnectorResult<quaint::prelude::ResultSet> {
        tracing::debug!(query_type = "raw_cmd", sql);
        self.0.query_raw(sql, params).await.map_err(quaint_err(url))
    }

    pub(super) async fn run_query_script(
        &mut self,
        sql: &str,
        url: &MysqlUrl,
        circumstances: BitFlags<super::Circumstances>,
    ) -> ConnectorResult<()> {
        let convert_error = |error: my::Error| match convert_server_error(circumstances, &error) {
            Some(e) => ConnectorError::from(e),
            None => quaint_err(url)(error.into()),
        };

        let mut conn = self.0.conn().lock().await;

        let mut result = sql.run(&mut *conn).await.map_err(convert_error)?;

        loop {
            match result.map(drop).await {
                Ok(_) => {
                    if result.is_empty() {
                        result.map(drop).await.map_err(convert_error)?;
                        return Ok(());
                    }
                }
                Err(e) => {
                    return Err(convert_error(e));
                }
            }
        }
    }

    pub(super) async fn apply_migration_script(
        &mut self,
        migration_name: &str,
        script: &str,
        circumstances: BitFlags<super::Circumstances>,
    ) -> ConnectorResult<()> {
        let convert_error = |migration_idx: usize, error: my::Error| {
            let position = format!(
                "Please check the query number {} from the migration file.",
                migration_idx + 1
            );

            let (code, error) = match (&error, convert_server_error(circumstances, &error)) {
                (my::Error::Server(se), Some(error)) => {
                    let message = format!("{}\n\n{}", error.message, position);
                    (Some(se.code), message)
                }
                (my::Error::Server(se), None) => {
                    let message = format!("{}\n\n{}", se.message, position);
                    (Some(se.code), message)
                }
                _ => (None, error.to_string()),
            };

            ConnectorError::user_facing(ApplyMigrationError {
                migration_name: migration_name.to_owned(),
                database_error_code: code.map(|c| c.to_string()).unwrap_or_else(|| String::from("none")),
                database_error: error,
            })
        };

        let mut conn = self.0.conn().lock().await;

        let mut migration_idx = 0_usize;

        let mut result = script
            .run(&mut *conn)
            .await
            .map_err(|e| convert_error(migration_idx, e))?;

        loop {
            match result.map(drop).await {
                Ok(_) => {
                    migration_idx += 1;

                    if result.is_empty() {
                        result.map(drop).await.map_err(|e| convert_error(migration_idx, e))?;
                        return Ok(());
                    }
                }
                Err(e) => {
                    return Err(convert_error(migration_idx, e));
                }
            }
        }
    }
}

fn quaint_err(url: &MysqlUrl) -> impl (Fn(quaint::error::Error) -> ConnectorError) + '_ {
    |err| crate::flavour::quaint_error_to_connector_error(err, &ConnectionInfo::Mysql(url.clone()))
}

fn convert_server_error(circumstances: BitFlags<super::Circumstances>, error: &my::Error) -> Option<KnownError> {
    if circumstances.contains(super::Circumstances::IsVitess) {
        match error {
            my::Error::Server(se) if se.code == 1317 => Some(KnownError::new(ForeignKeyCreationNotAllowed)),
            // sigh, this code is for unknown error, so we have the ddl
            // error and other stuff, such as typos in the same category...
            my::Error::Server(se) if se.code == 1105 && se.message == "direct DDL is disabled" => {
                Some(KnownError::new(DirectDdlNotAllowed))
            }
            _ => None,
        }
    } else {
        None
    }
}
