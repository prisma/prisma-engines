//! All the quaint-wrangling for the postgres connector should happen here.

use enumflags2::BitFlags;
use migration_connector::{ConnectorError, ConnectorResult, Namespaces};
use psl::PreviewFeature;
use quaint::{
    connector::{self, tokio_postgres::error::ErrorPosition, PostgresUrl},
    prelude::{ConnectionInfo, Queryable},
};
use sql_schema_describer::{postgres::PostgresSchemaExt, SqlSchema};
use user_facing_errors::{introspection_engine::DatabaseSchemaInconsistent, migration_engine::ApplyMigrationError};

use crate::sql_renderer::IteratorJoin;

pub(super) struct Connection(connector::PostgreSql);

impl Connection {
    pub(super) async fn new(url: url::Url) -> ConnectorResult<Connection> {
        let url = PostgresUrl::new(url).map_err(|err| {
            ConnectorError::user_facing(user_facing_errors::common::InvalidConnectionString {
                details: err.to_string(),
            })
        })?;

        let quaint = connector::PostgreSql::new(url.clone())
            .await
            .map_err(quaint_err(&url))?;

        let version = quaint.version().await.map_err(quaint_err(&url))?;

        if version.map(|v| v.starts_with("CockroachDB CCL v22.2")).unwrap_or(false) {
            // issue: https://github.com/prisma/prisma/issues/16909
            quaint
                .raw_cmd("SET enable_implicit_transaction_for_batch_statements=off")
                .await
                .map_err(quaint_err(&url))?;

            // Until at least version 22.2.5, enums are not type-sensitive without this.
            quaint
                .raw_cmd("SET use_declarative_schema_changer=off")
                .await
                .map_err(quaint_err(&url))?;
        }

        Ok(Connection(quaint))
    }

    #[tracing::instrument(skip(self, circumstances, params))]
    pub(super) async fn describe_schema(
        &mut self,
        circumstances: BitFlags<super::Circumstances>,
        params: &super::Params,
        namespaces: Option<Namespaces>,
    ) -> ConnectorResult<SqlSchema> {
        use sql_schema_describer::{postgres as describer, DescriberErrorKind, SqlSchemaDescriberBackend};
        let mut describer_circumstances: BitFlags<describer::Circumstances> = Default::default();

        if circumstances.contains(super::Circumstances::IsCockroachDb) {
            describer_circumstances |= describer::Circumstances::Cockroach;
        }

        if circumstances.contains(super::Circumstances::CockroachWithPostgresNativeTypes) {
            describer_circumstances |= describer::Circumstances::CockroachWithPostgresNativeTypes;
        }

        if circumstances.contains(super::Circumstances::CanPartitionTables) {
            describer_circumstances |= describer::Circumstances::CanPartitionTables;
        }

        let namespaces_vec = Namespaces::to_vec(namespaces, String::from(params.url.schema()));
        let namespaces_str: Vec<&str> = namespaces_vec.iter().map(AsRef::as_ref).collect();

        let mut schema = sql_schema_describer::postgres::SqlSchemaDescriber::new(&self.0, describer_circumstances)
            .describe(namespaces_str.as_slice())
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
        normalize_sql_schema(&mut schema, params.connector_params.preview_features);

        Ok(schema)
    }

    pub(super) async fn raw_cmd(&mut self, sql: &str, url: &PostgresUrl) -> ConnectorResult<()> {
        tracing::debug!(query_type = "raw_cmd", sql);
        self.0.raw_cmd(sql).await.map_err(quaint_err(url))
    }

    pub(super) async fn version(&mut self, url: &PostgresUrl) -> ConnectorResult<Option<String>> {
        tracing::debug!(query_type = "version");
        self.0.version().await.map_err(quaint_err(url))
    }

    pub(super) async fn query(
        &mut self,
        query: quaint::ast::Query<'_>,
        url: &PostgresUrl,
    ) -> ConnectorResult<quaint::prelude::ResultSet> {
        use quaint::visitor::Visitor;
        let (sql, params) = quaint::visitor::Postgres::build(query).unwrap();
        self.query_raw(&sql, &params, url).await
    }

    pub(super) async fn query_raw(
        &self,
        sql: &str,
        params: &[quaint::prelude::Value<'_>],
        url: &PostgresUrl,
    ) -> ConnectorResult<quaint::prelude::ResultSet> {
        tracing::debug!(query_type = "query_raw", sql, ?params);
        self.0.query_raw(sql, params).await.map_err(quaint_err(url))
    }

    pub(super) async fn apply_migration_script(&mut self, migration_name: &str, script: &str) -> ConnectorResult<()> {
        tracing::debug!(query_type = "raw_cmd", script);
        let client = self.0.client();

        match client.simple_query(script).await {
            Ok(_) => Ok(()),
            Err(err) => {
                let (database_error_code, database_error): (Option<&str>, _) = if let Some(db_error) = err.as_db_error()
                {
                    let position = if let Some(ErrorPosition::Original(position)) = db_error.position() {
                        let mut previous_lines = [""; 5];
                        let mut byte_index = 0;
                        let mut error_position = String::new();

                        for (line_idx, line) in script.lines().enumerate() {
                            // Line numbers start at 1, not 0.
                            let line_number = line_idx + 1;
                            byte_index += line.len() + 1; // + 1 for the \n character.

                            if *position as usize <= byte_index {
                                let numbered_lines = previous_lines
                                    .iter()
                                    .enumerate()
                                    .filter_map(|(idx, line)| {
                                        line_number
                                            .checked_sub(previous_lines.len() - idx)
                                            .map(|idx| (idx, line))
                                    })
                                    .map(|(idx, line)| {
                                        format!(
                                            "\x1b[1m{:>3}\x1b[0m{}{}",
                                            idx,
                                            if line.is_empty() { "" } else { " " },
                                            line
                                        )
                                    })
                                    .join("\n");

                                error_position = format!(
                                    "\n\nPosition:\n{numbered_lines}\n\x1b[1m{line_number:>3}\x1b[1;31m {line}\x1b[0m"
                                );
                                break;
                            } else {
                                previous_lines = [
                                    previous_lines[1],
                                    previous_lines[2],
                                    previous_lines[3],
                                    previous_lines[4],
                                    line,
                                ];
                            }
                        }

                        error_position
                    } else {
                        String::new()
                    };

                    let database_error = format!("{db_error}{position}\n\n{db_error:?}");

                    (Some(db_error.code().code()), database_error)
                } else {
                    (err.code().map(|c| c.code()), err.to_string())
                };

                Err(ConnectorError::user_facing(ApplyMigrationError {
                    migration_name: migration_name.to_owned(),
                    database_error_code: database_error_code.unwrap_or("none").to_owned(),
                    database_error,
                }))
            }
        }
    }
}

fn normalize_sql_schema(schema: &mut SqlSchema, preview_features: BitFlags<PreviewFeature>) {
    if !preview_features.contains(PreviewFeature::PostgresqlExtensions) {
        let pg_ext: &mut PostgresSchemaExt = schema.downcast_connector_data_mut();
        pg_ext.clear_extensions();
    }
}

fn quaint_err(url: &PostgresUrl) -> impl (Fn(quaint::error::Error) -> ConnectorError) + '_ {
    |err| crate::flavour::quaint_error_to_connector_error(err, &ConnectionInfo::Postgres(url.clone()))
}
