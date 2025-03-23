//! All the quaint-wrangling for the mysql connector should happen here.

pub mod shadow_db;

use enumflags2::BitFlags;
use quaint::{
    connector::{
        self,
        mysql_async::{self as my, prelude::Query},
        MysqlUrl,
    },
    prelude::{ColumnType, ConnectionInfo, NativeConnectionInfo, Queryable},
};
use schema_connector::{ConnectorError, ConnectorResult};
use sql_schema_describer::{DescriberErrorKind, SqlSchema};
use user_facing_errors::{
    schema_engine::DatabaseSchemaInconsistent,
    schema_engine::{ApplyMigrationError, DirectDdlNotAllowed, ForeignKeyCreationNotAllowed},
    KnownError,
};

pub struct Connection(connector::Mysql);

impl Connection {
    pub async fn new(url: url::Url) -> ConnectorResult<Connection> {
        let url = MysqlUrl::new(url).map_err(|err| {
            ConnectorError::user_facing(user_facing_errors::common::InvalidConnectionString {
                details: err.to_string(),
            })
        })?;
        Ok(Connection(
            connector::Mysql::new(url.clone()).await.map_err(quaint_err(&url))?,
        ))
    }

    #[tracing::instrument(skip(self, circumstances, params))]
    pub async fn describe_schema(
        &mut self,
        circumstances: BitFlags<super::Circumstances>,
        params: &super::Params,
    ) -> ConnectorResult<SqlSchema> {
        use sql_schema_describer::{mysql as describer, SqlSchemaDescriberBackend};
        let mut describer_circumstances: BitFlags<describer::Circumstances> = Default::default();

        if circumstances.contains(super::Circumstances::IsMariadb) {
            describer_circumstances |= describer::Circumstances::MariaDb;
        }

        if circumstances.contains(super::Circumstances::IsMysql56) {
            describer_circumstances |= describer::Circumstances::MySql56;
        }

        if circumstances.contains(super::Circumstances::IsMysql57) {
            describer_circumstances |= describer::Circumstances::MySql57;
        }

        if circumstances.contains(super::Circumstances::CheckConstraints)
            && !describer_circumstances.intersects(
                describer::Circumstances::MySql56
                    | describer::Circumstances::MySql57
                    | describer::Circumstances::MariaDb,
            )
        {
            // MySQL 8.0.16 and above supports check constraints.
            // MySQL 5.6 and 5.7 do not have a CHECK_CONSTRAINTS table we can query.
            // MariaDB, although it supports check constraints, adds them unexpectedly.
            // E.g., MariaDB 10 adds the `json_valid(\`Priv\`)` check constraint on every JSON column;
            // this creates a noisy, unexpected diff when comparing the introspected schema with the prisma schema.
            describer_circumstances |= describer::Circumstances::CheckConstraints;
        }

        let mut schema = sql_schema_describer::mysql::SqlSchemaDescriber::new(&self.0, describer_circumstances)
            .describe(&[params.url.dbname()])
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

    pub async fn raw_cmd(&mut self, sql: &str, url: &MysqlUrl) -> ConnectorResult<()> {
        tracing::debug!(query_type = "raw_cmd", sql);
        self.0.raw_cmd(sql).await.map_err(quaint_err(url))
    }

    pub async fn version(&mut self, url: &MysqlUrl) -> ConnectorResult<Option<String>> {
        tracing::debug!(query_type = "version");
        self.0.version().await.map_err(quaint_err(url))
    }

    pub async fn query(
        &mut self,
        query: quaint::ast::Query<'_>,
        url: &MysqlUrl,
    ) -> ConnectorResult<quaint::prelude::ResultSet> {
        use quaint::visitor::Visitor;
        let (sql, params) = quaint::visitor::Mysql::build(query).unwrap();
        self.query_raw(&sql, &params, url).await
    }

    pub async fn query_raw(
        &mut self,
        sql: &str,
        params: &[quaint::prelude::Value<'_>],
        url: &MysqlUrl,
    ) -> ConnectorResult<quaint::prelude::ResultSet> {
        tracing::debug!(query_type = "query_raw", sql, ?params);
        self.0.query_raw(sql, params).await.map_err(quaint_err(url))
    }

    pub async fn describe_query(
        &self,
        sql: &str,
        url: &MysqlUrl,
        circumstances: BitFlags<super::Circumstances>,
    ) -> ConnectorResult<quaint::connector::DescribedQuery> {
        tracing::debug!(query_type = "describe_query", sql);
        let mut parsed = self.0.describe_query(sql).await.map_err(quaint_err(url))?;

        if circumstances.contains(super::Circumstances::IsMysql56)
            || circumstances.contains(super::Circumstances::IsMysql57)
        {
            parsed.parameters = parsed
                .parameters
                .into_iter()
                .map(|p| p.set_typ(ColumnType::Unknown))
                .collect();

            return Ok(parsed);
        }

        Ok(parsed)
    }

    pub async fn apply_migration_script(
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

        tracing::debug!(sql = script, query_type = "raw_cmd");
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
    |err| {
        crate::flavour::quaint_error_to_connector_error(
            err,
            &ConnectionInfo::Native(NativeConnectionInfo::Mysql(url.clone())),
        )
    }
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
