//! SQLite description.
use super::*;
use failure::_core::convert::TryInto;
use log::debug;
use quaint::{ast::ParameterizedValue, prelude::Queryable};
use std::collections::HashMap;
use std::sync::Arc;

pub struct SqlSchemaDescriber {
    conn: Arc<dyn Queryable + Send + Sync + 'static>,
}

#[async_trait::async_trait]
impl super::SqlSchemaDescriberBackend for SqlSchemaDescriber {
    async fn list_databases(&self) -> SqlSchemaDescriberResult<Vec<String>> {
        let databases = self.get_databases().await;
        Ok(databases)
    }

    async fn get_metadata(&self, schema: &str) -> SqlSchemaDescriberResult<SQLMetadata> {
        let count = self.get_table_names(&schema).await.len();
        let size = self.get_size(&schema).await;
        Ok(SQLMetadata {
            table_count: count,
            size_in_bytes: size,
        })
    }

    async fn describe(&self, schema: &str) -> SqlSchemaDescriberResult<SqlSchema> {
        debug!("describing schema '{}'", schema);
        let table_names: Vec<String> = self.get_table_names(schema).await;

        let mut tables = Vec::with_capacity(table_names.len());

        for table_name in table_names.iter().filter(|table| !is_system_table(&table)) {
            tables.push(self.get_table(schema, table_name).await)
        }

        Ok(SqlSchema {
            // There's no enum type in SQLite.
            enums: vec![],
            // There are no sequences in SQLite.
            sequences: vec![],
            tables: tables,
        })
    }
}

impl SqlSchemaDescriber {
    /// Constructor.
    pub fn new(conn: Arc<dyn Queryable + Send + Sync + 'static>) -> SqlSchemaDescriber {
        SqlSchemaDescriber { conn }
    }

    async fn get_databases(&self) -> Vec<String> {
        debug!("Getting databases");
        let sql = "PRAGMA database_list;";
        let rows = self.conn.query_raw(sql, &[]).await.expect("get schema names ");
        let names = rows
            .into_iter()
            .map(|row| {
                row.get("file")
                    .and_then(|x| x.to_string())
                    .and_then(|x| x.split("/").last().map(|x| x.to_string()))
                    .expect("convert schema names")
            })
            .collect();

        debug!("Found schema names: {:?}", names);
        names
    }

    async fn get_table_names(&self, schema: &str) -> Vec<String> {
        let sql = format!(r#"SELECT name FROM "{}".sqlite_master WHERE type='table'"#, schema);
        debug!("describing table names with query: '{}'", sql);
        let result_set = self.conn.query_raw(&sql, &[]).await.expect("get table names");
        let names = result_set
            .into_iter()
            .map(|row| row.get("name").and_then(|x| x.to_string()).unwrap())
            .filter(|n| n != "sqlite_sequence")
            .collect();
        debug!("Found table names: {:?}", names);
        names
    }

    async fn get_size(&self, _schema: &str) -> usize {
        debug!("Getting db size");
        let sql = format!(r#"SELECT page_count * page_size as size FROM pragma_page_count(), pragma_page_size();"#);
        let result = self.conn.query_raw(&sql, &[]).await.expect("get db size ");
        let size: i64 = result
            .first()
            .map(|row| row.get("size").and_then(|x| x.as_i64()).unwrap_or(0))
            .unwrap();

        size.try_into().unwrap()
    }

    async fn get_table(&self, schema: &str, name: &str) -> Table {
        debug!("describing table '{}' in schema '{}", name, schema);
        let (columns, primary_key) = self.get_columns(schema, name).await;
        let foreign_keys = self.get_foreign_keys(schema, name).await;
        let indices = self.get_indices(schema, name).await;
        Table {
            name: name.to_string(),
            columns,
            indices,
            primary_key,
            foreign_keys,
        }
    }

    async fn get_columns(&self, schema: &str, table: &str) -> (Vec<Column>, Option<PrimaryKey>) {
        let sql = format!(r#"PRAGMA "{}".table_info ("{}")"#, schema, table);
        debug!("describing table columns, query: '{}'", sql);
        let result_set = self.conn.query_raw(&sql, &[]).await.unwrap();
        let mut pk_cols: HashMap<i64, String> = HashMap::new();
        let mut cols: Vec<Column> = result_set
            .into_iter()
            .map(|row| {
                debug!("Got column row {:?}", row);
                let is_required = row.get("notnull").and_then(|x| x.as_bool()).expect("notnull");

                let arity = if is_required {
                    ColumnArity::Required
                } else {
                    ColumnArity::Nullable
                };
                let tpe = get_column_type(&row.get("type").and_then(|x| x.to_string()).expect("type"), arity);

                let default = match row.get("dflt_value") {
                    None => None,
                    Some(ParameterizedValue::Null) => None,
                    Some(ParameterizedValue::Text(cow_string)) => {
                        let default_string = cow_string.to_string();

                        if default_string.to_lowercase() == "null" {
                            None
                        } else {
                            Some(match &tpe.family {
                                ColumnTypeFamily::Int => match parse_int(&default_string) {
                                    Some(int_value) => DefaultValue::VALUE(int_value),
                                    None => DefaultValue::DBGENERATED(default_string),
                                },
                                ColumnTypeFamily::Float => match parse_float(&default_string) {
                                    Some(float_value) => DefaultValue::VALUE(float_value),
                                    None => DefaultValue::DBGENERATED(default_string),
                                },
                                ColumnTypeFamily::Boolean => match parse_int(&default_string) {
                                    Some(PrismaValue::Int(1)) => DefaultValue::VALUE(PrismaValue::Boolean(true)),
                                    Some(PrismaValue::Int(0)) => DefaultValue::VALUE(PrismaValue::Boolean(false)),
                                    _ => match parse_bool(&default_string) {
                                        Some(bool_value) => DefaultValue::VALUE(bool_value),
                                        None => DefaultValue::DBGENERATED(default_string),
                                    },
                                },
                                ColumnTypeFamily::String => {
                                    DefaultValue::VALUE(PrismaValue::String(unquote_string(default_string)))
                                }
                                ColumnTypeFamily::DateTime => match default_string.to_lowercase()
                                    == "current_timestamp".to_string()
                                    || default_string.to_lowercase() == "datetime(\'now\')".to_string()
                                    || default_string.to_lowercase() == "datetime(\'now\', \'localtime\')".to_string()
                                {
                                    true => DefaultValue::NOW,
                                    false => DefaultValue::DBGENERATED(default_string),
                                },
                                ColumnTypeFamily::Binary => DefaultValue::DBGENERATED(default_string),
                                ColumnTypeFamily::Json => DefaultValue::DBGENERATED(default_string),
                                ColumnTypeFamily::Uuid => DefaultValue::DBGENERATED(default_string),
                                ColumnTypeFamily::Geometric => DefaultValue::DBGENERATED(default_string),
                                ColumnTypeFamily::LogSequenceNumber => DefaultValue::DBGENERATED(default_string),
                                ColumnTypeFamily::TextSearch => DefaultValue::DBGENERATED(default_string),
                                ColumnTypeFamily::TransactionId => DefaultValue::DBGENERATED(default_string),
                                ColumnTypeFamily::Enum(_) => DefaultValue::VALUE(PrismaValue::Enum(default_string)),
                                ColumnTypeFamily::Unsupported(_) => DefaultValue::DBGENERATED(default_string),
                            })
                        }
                    }
                    Some(_) => None,
                };

                let pk_col = row.get("pk").and_then(|x| x.as_i64()).expect("primary key");
                let col = Column {
                    name: row.get("name").and_then(|x| x.to_string()).expect("name"),
                    tpe,
                    default,
                    auto_increment: false,
                };
                if pk_col > 0 {
                    pk_cols.insert(pk_col, col.name.clone());
                }

                debug!(
                    "Found column '{}', type: '{:?}', default: {:?}, primary key: {}",
                    col.name,
                    col.tpe,
                    col.default,
                    pk_col > 0
                );

                col
            })
            .collect();
        cols.sort_unstable_by_key(|col| col.name.clone());

        let primary_key = match pk_cols.is_empty() {
            true => {
                debug!("Determined that table has no primary key");
                None
            }
            false => {
                let mut columns: Vec<String> = vec![];
                let mut col_idxs: Vec<&i64> = pk_cols.keys().collect();
                col_idxs.sort_unstable();
                for i in col_idxs {
                    columns.push(pk_cols[i].clone());
                }

                //Integer Id columns are always implemented with either row id or autoincrement
                if pk_cols.len() == 1 {
                    let pk_col = &columns[0];
                    for col in cols.iter_mut() {
                        if &col.name == pk_col && &col.tpe.raw.to_lowercase() == "integer" {
                            debug!(
                                "Detected that the primary key column corresponds to rowid and \
                                 is auto incrementing"
                            );
                            col.auto_increment = true;
                        }
                    }
                }

                debug!("Determined that table has primary key with columns {:?}", columns);
                Some(PrimaryKey {
                    columns,
                    sequence: None,
                })
            }
        };

        (cols, primary_key)
    }

    async fn get_foreign_keys(&self, schema: &str, table: &str) -> Vec<ForeignKey> {
        struct IntermediateForeignKey {
            pub columns: HashMap<i64, String>,
            pub referenced_table: String,
            pub referenced_columns: HashMap<i64, String>,
            pub on_delete_action: ForeignKeyAction,
        }

        let sql = format!(r#"PRAGMA "{}".foreign_key_list("{}");"#, schema, table);
        debug!("describing table foreign keys, SQL: '{}'", sql);
        let result_set = self.conn.query_raw(&sql, &[]).await.expect("querying for foreign keys");

        // Since one foreign key with multiple columns will be represented here as several
        // rows with the same ID, we have to use an intermediate representation that gets
        // translated into the real foreign keys in another pass
        let mut intermediate_fks: HashMap<i64, IntermediateForeignKey> = HashMap::new();
        for row in result_set.into_iter() {
            debug!("got FK description row {:?}", row);
            let id = row.get("id").and_then(|x| x.as_i64()).expect("id");
            let seq = row.get("seq").and_then(|x| x.as_i64()).expect("seq");
            let column = row.get("from").and_then(|x| x.to_string()).expect("from");
            let referenced_column = row.get("to").and_then(|x| x.to_string()).expect("to");
            let referenced_table = row.get("table").and_then(|x| x.to_string()).expect("table");
            match intermediate_fks.get_mut(&id) {
                Some(fk) => {
                    fk.columns.insert(seq, column);
                    fk.referenced_columns.insert(seq, referenced_column);
                }
                None => {
                    let mut columns: HashMap<i64, String> = HashMap::new();
                    columns.insert(seq, column);
                    let mut referenced_columns: HashMap<i64, String> = HashMap::new();
                    referenced_columns.insert(seq, referenced_column);
                    let on_delete_action = match row
                        .get("on_delete")
                        .and_then(|x| x.to_string())
                        .expect("on_delete")
                        .to_lowercase()
                        .as_str()
                    {
                        "no action" => ForeignKeyAction::NoAction,
                        "restrict" => ForeignKeyAction::Restrict,
                        "set null" => ForeignKeyAction::SetNull,
                        "set default" => ForeignKeyAction::SetDefault,
                        "cascade" => ForeignKeyAction::Cascade,
                        s @ _ => panic!(format!("Unrecognized on delete action '{}'", s)),
                    };
                    let fk = IntermediateForeignKey {
                        columns,
                        referenced_table,
                        referenced_columns,
                        on_delete_action,
                    };
                    intermediate_fks.insert(id, fk);
                }
            };
        }

        let mut fks: Vec<ForeignKey> = intermediate_fks
            .values()
            .into_iter()
            .map(|intermediate_fk| {
                let mut column_keys: Vec<&i64> = intermediate_fk.columns.keys().collect();
                column_keys.sort();
                let mut columns: Vec<String> = vec![];
                columns.reserve(column_keys.len());
                for i in column_keys {
                    columns.push(intermediate_fk.columns[i].to_owned());
                }

                let mut referenced_column_keys: Vec<&i64> = intermediate_fk.referenced_columns.keys().collect();
                referenced_column_keys.sort();
                let mut referenced_columns: Vec<String> = vec![];
                referenced_columns.reserve(referenced_column_keys.len());
                for i in referenced_column_keys {
                    referenced_columns.push(intermediate_fk.referenced_columns[i].to_owned());
                }

                let fk = ForeignKey {
                    columns,
                    referenced_table: intermediate_fk.referenced_table.to_owned(),
                    referenced_columns,
                    on_delete_action: intermediate_fk.on_delete_action.to_owned(),

                    // Not relevant in SQLite since we cannot ALTER or DROP foreign keys by
                    // constraint name.
                    constraint_name: None,
                };
                debug!("Detected foreign key {:?}", fk);
                fk
            })
            .collect();

        fks.sort_unstable_by_key(|fk| fk.columns.clone());

        fks
    }

    async fn get_indices(&self, schema: &str, table: &str) -> Vec<Index> {
        let sql = format!(r#"PRAGMA "{}".index_list("{}");"#, schema, table);
        debug!("describing table indices, SQL: '{}'", sql);
        let result_set = self.conn.query_raw(&sql, &[]).await.expect("querying for indices");
        debug!("Got indices description results: {:?}", result_set);

        let mut indices = Vec::new();
        let filtered_rows = result_set
            .into_iter()
            // Exclude primary keys, they are inferred separately.
            .filter(|row| row.get("origin").and_then(|origin| origin.as_str()).unwrap() != "pk");

        for row in filtered_rows {
            let is_unique = row.get("unique").and_then(|x| x.as_bool()).expect("get unique");
            let name = row.get("name").and_then(|x| x.to_string()).expect("get name");
            let mut index = Index {
                name: name.clone(),
                tpe: match is_unique {
                    true => IndexType::Unique,
                    false => IndexType::Normal,
                },
                columns: vec![],
            };

            let sql = format!(r#"PRAGMA "{}".index_info("{}");"#, schema, name);
            debug!("describing table index '{}', SQL: '{}'", name, sql);
            let result_set = self.conn.query_raw(&sql, &[]).await.expect("querying for index info");
            debug!("Got index description results: {:?}", result_set);
            for row in result_set.into_iter() {
                let pos = row.get("seqno").and_then(|x| x.as_i64()).expect("get seqno") as usize;
                let col_name = row.get("name").and_then(|x| x.to_string()).expect("get name");
                if index.columns.len() <= pos {
                    index.columns.resize(pos + 1, "".to_string());
                }
                index.columns[pos] = col_name;
            }

            indices.push(index)
        }

        indices
    }
}

fn get_column_type(tpe: &str, arity: ColumnArity) -> ColumnType {
    let tpe_lower = tpe.to_lowercase();

    let family = match tpe_lower.as_ref() {
        // SQLite only has a few native data types: https://www.sqlite.org/datatype3.html
        // It's tolerant though, and you can assign any data type you like to columns
        "int" => ColumnTypeFamily::Int,
        "integer" => ColumnTypeFamily::Int,
        "real" => ColumnTypeFamily::Float,
        "float" => ColumnTypeFamily::Float,
        "serial" => ColumnTypeFamily::Int,
        "boolean" => ColumnTypeFamily::Boolean,
        "text" => ColumnTypeFamily::String,
        s if s.contains("char") => ColumnTypeFamily::String,
        s if s.contains("numeric") => ColumnTypeFamily::Float,
        "date" => ColumnTypeFamily::DateTime,
        "datetime" => ColumnTypeFamily::DateTime,
        "timestamp" => ColumnTypeFamily::DateTime,
        "binary" => ColumnTypeFamily::Binary,
        "double" => ColumnTypeFamily::Float,
        "binary[]" => ColumnTypeFamily::Binary,
        "boolean[]" => ColumnTypeFamily::Boolean,
        "date[]" => ColumnTypeFamily::DateTime,
        "datetime[]" => ColumnTypeFamily::DateTime,
        "timestamp[]" => ColumnTypeFamily::DateTime,
        "double[]" => ColumnTypeFamily::Float,
        "float[]" => ColumnTypeFamily::Float,
        "int[]" => ColumnTypeFamily::Int,
        "integer[]" => ColumnTypeFamily::Int,
        "text[]" => ColumnTypeFamily::String,
        data_type => ColumnTypeFamily::Unsupported(data_type.into()),
    };
    ColumnType {
        raw: tpe.to_string(),
        family: family,
        arity,
    }
}

/// Returns whether a table is one of the SQLite system tables.
fn is_system_table(table_name: &str) -> bool {
    SQLITE_SYSTEM_TABLES
        .iter()
        .any(|system_table| table_name == *system_table)
}

/// See https://www.sqlite.org/fileformat2.html
const SQLITE_SYSTEM_TABLES: &[&str] = &[
    "sqlite_sequence",
    "sqlite_stat1",
    "sqlite_stat2",
    "sqlite_stat3",
    "sqlite_stat4",
];
