use crate::BitFlags;
use crate::flavour::quaint_error_to_connector_error;
use psl::PreviewFeature;
use quaint::connector::ExternalConnector;
use schema_connector::{ConnectorError, ConnectorResult};
use sql_schema_describer::SqlSchema;
use std::sync::Arc;

pub struct State {
    connection: Connection,
    params: Params,
    preview_features: BitFlags<PreviewFeature>,
}

impl State {
    pub fn new(adapter: Arc<dyn ExternalConnector>, preview_features: BitFlags<PreviewFeature>) -> Self {
        Self {
            preview_features,
            connection: Connection { adapter },
            params: Params,
        }
    }
}

pub struct Params;

pub struct Connection {
    adapter: Arc<dyn ExternalConnector>,
}

impl Connection {
    pub fn as_connector(&self) -> &Arc<dyn ExternalConnector> {
        &self.adapter
    }

    pub async fn raw_cmd(&self, sql: &str) -> ConnectorResult<()> {
        tracing::debug!(query_type = "raw_cmd", sql);
        self.adapter.raw_cmd(sql).await.map_err(convert_error)
    }

    pub async fn query(&self, query: quaint::ast::Query<'_>) -> ConnectorResult<quaint::prelude::ResultSet> {
        use quaint::visitor::Visitor;
        let (sql, params) = quaint::visitor::SurrealDb::build(query)
            .map_err(|e| ConnectorError::from_msg(format!("Failed to build SurrealQL query: {e}")))?;
        self.query_raw(&sql, &params).await
    }

    pub async fn query_raw(
        &self,
        sql: &str,
        params: &[quaint::prelude::Value<'_>],
    ) -> ConnectorResult<quaint::prelude::ResultSet> {
        tracing::debug!(query_type = "query_raw", sql);
        self.adapter.query_raw(sql, params).await.map_err(convert_error)
    }

    pub async fn version(&self) -> ConnectorResult<Option<String>> {
        self.adapter.version().await.map_err(convert_error)
    }

    pub async fn describe_query(
        &self,
        sql: &str,
        _params: &Params,
    ) -> ConnectorResult<quaint::connector::DescribedQuery> {
        tracing::debug!(query_type = "describe_query", sql);
        self.adapter.describe_query(sql).await.map_err(convert_error)
    }

    pub async fn apply_migration_script(&self, _migration_name: &str, script: &str) -> ConnectorResult<()> {
        self.adapter.execute_script(script).await.map_err(convert_error)
    }

    pub async fn reset(&self, _params: &Params) -> ConnectorResult<()> {
        let tables = self.list_tables().await?;

        if tables.is_empty() {
            return Ok(());
        }

        let drop_statements: Vec<String> = tables
            .iter()
            .map(|table| format!("REMOVE TABLE `{table}`"))
            .collect();

        self.adapter
            .execute_script(&drop_statements.join("; "))
            .await
            .map_err(convert_error)
    }

    pub async fn list_tables(&self) -> ConnectorResult<Vec<String>> {
        let result = self
            .adapter
            .query_raw("INFO FOR DB", &[])
            .await
            .map_err(convert_error)?;

        let mut tables = Vec::new();

        // INFO FOR DB returns a result set where first row contains a "tables" field
        // with a JSON object mapping table_name => definition_string.
        for row in result.into_iter() {
            if let Some(tables_val) = row.get("tables") {
                if let Some(s) = tables_val.to_string() {
                    if let Ok(obj) = serde_json::from_str::<serde_json::Value>(&s) {
                        if let Some(map) = obj.as_object() {
                            for key in map.keys() {
                                tables.push(key.clone());
                            }
                        }
                    }
                }
            }
        }

        Ok(tables)
    }

    async fn dispose(&self) -> ConnectorResult<()> {
        self.adapter.dispose().await.map_err(convert_error)
    }
}

pub fn connect_to_shadow_db() -> ConnectorResult<Connection> {
    Err(ConnectorError::from_msg(
        "SurrealDB shadow DB must be provided through an external factory".to_owned(),
    ))
}

pub async fn create_database(state: &State) -> ConnectorResult<String> {
    state
        .connection
        .raw_cmd("DEFINE NAMESPACE IF NOT EXISTS prisma; DEFINE DATABASE IF NOT EXISTS prisma")
        .await?;
    Ok("Database created via DEFINE NAMESPACE + DEFINE DATABASE".to_owned())
}

pub async fn drop_database(state: &State) -> ConnectorResult<()> {
    state
        .connection
        .raw_cmd("REMOVE DATABASE IF EXISTS prisma")
        .await
}

pub async fn ensure_connection_validity(state: &mut State) -> ConnectorResult<()> {
    let (connection, _) = get_connection_and_params(state)?;
    connection.version().await?;
    Ok(())
}

pub async fn introspect(state: &mut State) -> ConnectorResult<SqlSchema> {
    use sql_schema_describer::*;

    let conn = &state.connection;
    let table_names = conn.list_tables().await?;

    let mut schema = SqlSchema::default();
    let ns_id = schema.push_namespace("default".to_owned());

    for table_name in &table_names {
        let result = conn
            .query_raw(&format!("INFO FOR TABLE `{table_name}`"), &[])
            .await?;

        let table_id = schema.push_table(table_name.clone(), ns_id, None);

        for row in result.into_iter() {
            // Parse fields from INFO FOR TABLE response
            if let Some(fields_val) = row.get("fields") {
                if let Some(s) = fields_val.to_string() {
                    if let Ok(obj) = serde_json::from_str::<serde_json::Value>(&s) {
                        if let Some(map) = obj.as_object() {
                            for (field_name, def_val) in map {
                                let def_str = def_val.as_str().unwrap_or("");
                                let col_type = parse_surreal_field_type(def_str);
                                schema.push_table_column(table_id, Column {
                                    name: field_name.clone(),
                                    tpe: col_type,
                                    auto_increment: false,
                                    description: None,
                                });
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(schema)
}

fn parse_surreal_field_type(def: &str) -> sql_schema_describer::ColumnType {
    use sql_schema_describer::*;

    // Extract type from "DEFINE FIELD name ON table TYPE <type> ..."
    let type_str = def
        .split("TYPE ")
        .nth(1)
        .unwrap_or("string")
        .split_whitespace()
        .next()
        .unwrap_or("string")
        .trim_start_matches("option<")
        .trim_end_matches('>');

    let family = match type_str {
        "bool" => ColumnTypeFamily::Boolean,
        "int" => ColumnTypeFamily::Int,
        "float" => ColumnTypeFamily::Float,
        "decimal" => ColumnTypeFamily::Decimal,
        "string" => ColumnTypeFamily::String,
        "datetime" => ColumnTypeFamily::DateTime,
        "bytes" => ColumnTypeFamily::Binary,
        "object" | "record" => ColumnTypeFamily::Json,
        "uuid" => ColumnTypeFamily::Uuid,
        _ => ColumnTypeFamily::String,
    };

    let arity = if def.contains("| NONE") || def.contains("option<") {
        ColumnArity::Nullable
    } else {
        ColumnArity::Required
    };

    ColumnType {
        full_data_type: type_str.to_owned(),
        family,
        arity,
        native_type: None,
    }
}

pub fn get_connection_and_params(state: &mut State) -> ConnectorResult<(&Connection, &Params)> {
    Ok((&state.connection, &state.params))
}

pub fn set_preview_features(state: &mut State, features: BitFlags<PreviewFeature>) {
    state.preview_features = features;
}

pub fn get_preview_features(state: &State) -> psl::PreviewFeatures {
    state.preview_features
}

pub fn get_shadow_db_url(_state: &State) -> Option<&str> {
    None
}

pub async fn dispose(state: &State) -> ConnectorResult<()> {
    state.connection.dispose().await
}

fn convert_error(err: quaint::error::Error) -> ConnectorError {
    quaint_error_to_connector_error(err, None)
}
