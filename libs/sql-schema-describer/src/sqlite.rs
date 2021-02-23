//! SQLite description.
use super::*;
use crate::{getters::Getter, parsers::Parser};
use quaint::{ast::Value, prelude::Queryable, single::Quaint};
use std::{borrow::Cow, collections::HashMap, convert::TryInto};
use tracing::trace;

#[derive(Debug)]
pub struct SqlSchemaDescriber {
    conn: Quaint,
}

#[async_trait::async_trait]
impl super::SqlSchemaDescriberBackend for SqlSchemaDescriber {
    async fn list_databases(&self) -> DescriberResult<Vec<String>> {
        Ok(self.get_databases().await?)
    }

    async fn get_metadata(&self, schema: &str) -> DescriberResult<SQLMetadata> {
        let table_count = self.get_table_names(&schema).await?.len();
        let size_in_bytes = self.get_size().await?;

        Ok(SQLMetadata {
            table_count,
            size_in_bytes,
        })
    }

    #[tracing::instrument]
    async fn describe(&self, schema: &str) -> DescriberResult<SqlSchema> {
        let table_names: Vec<String> = self.get_table_names(schema).await?;

        let mut tables = Vec::with_capacity(table_names.len());

        for table_name in table_names.iter().filter(|table| !is_system_table(&table)) {
            tables.push(self.get_table(schema, table_name).await?)
        }

        // sqlite allows foreign key definitions without specifying the referenced columns, it then assumes the pk is used
        let mut foreign_keys_without_referenced_columns = vec![];

        for (table_index, table) in tables.iter().enumerate() {
            for (fk_index, foreign_key) in table.foreign_keys.iter().enumerate() {
                if foreign_key.referenced_columns.is_empty() {
                    let referenced_table = tables.iter().find(|t| t.name == foreign_key.referenced_table).unwrap();
                    let referenced_pk = referenced_table.primary_key.as_ref().unwrap();
                    foreign_keys_without_referenced_columns.push((table_index, fk_index, referenced_pk.columns.clone()))
                }
            }
        }

        for (table_index, fk_index, columns) in foreign_keys_without_referenced_columns {
            tables[table_index].foreign_keys[fk_index].referenced_columns = columns
        }

        let views = self.get_views().await?;

        Ok(SqlSchema {
            // There's no enum type in SQLite.
            enums: vec![],
            // There are no sequences in SQLite.
            sequences: vec![],
            // There are no procedures in SQLite (phew).
            procedures: vec![],
            tables,
            views,
        })
    }

    #[tracing::instrument]
    async fn version(&self, schema: &str) -> DescriberResult<Option<String>> {
        Ok(self.conn.version().await?)
    }
}

impl Parser for SqlSchemaDescriber {}

impl SqlSchemaDescriber {
    /// Constructor.
    pub fn new(conn: Quaint) -> SqlSchemaDescriber {
        SqlSchemaDescriber { conn }
    }

    #[tracing::instrument]
    async fn get_databases(&self) -> DescriberResult<Vec<String>> {
        let sql = "PRAGMA database_list;";
        let rows = self.conn.query_raw(sql, &[]).await?;
        let names = rows
            .into_iter()
            .map(|row| {
                row.get("file")
                    .and_then(|x| x.to_string())
                    .and_then(|x| x.split('/').last().map(|x| x.to_string()))
                    .expect("convert schema names")
            })
            .collect();

        trace!("Found schema names: {:?}", names);

        Ok(names)
    }

    async fn get_table_names(&self, _schema: &str) -> DescriberResult<Vec<String>> {
        let sql = r#"SELECT name FROM sqlite_master WHERE type='table' ORDER BY name ASC"#;
        trace!("describing table names with query: '{}'", sql);

        let result_set = self.conn.query_raw(&sql, &[]).await?;

        let names = result_set
            .into_iter()
            .map(|row| row.get("name").and_then(|x| x.to_string()).unwrap())
            .filter(|n| n != "sqlite_sequence")
            .collect();

        trace!("Found table names: {:?}", names);

        Ok(names)
    }

    #[tracing::instrument]
    async fn get_size(&self) -> DescriberResult<usize> {
        let sql = r#"SELECT page_count * page_size as size FROM pragma_page_count(), pragma_page_size();"#;
        let result = self.conn.query_raw(&sql, &[]).await?;
        let size: i64 = result
            .first()
            .map(|row| row.get("size").and_then(|x| x.as_i64()).unwrap_or(0))
            .unwrap();

        Ok(size.try_into().unwrap())
    }

    #[tracing::instrument]
    async fn get_table(&self, schema: &str, name: &str) -> DescriberResult<Table> {
        let (columns, primary_key) = self.get_columns(name).await?;
        let foreign_keys = self.get_foreign_keys(name).await?;
        let indices = self.get_indices(name, &columns).await?;

        Ok(Table {
            name: name.to_string(),
            columns,
            indices,
            primary_key,
            foreign_keys,
        })
    }

    #[tracing::instrument]
    async fn get_views(&self) -> DescriberResult<Vec<View>> {
        let sql = "SELECT name AS view_name, sql AS view_sql FROM sqlite_master WHERE type = 'view'";
        let result_set = self.conn.query_raw(sql, &[]).await?;
        let mut views = Vec::with_capacity(result_set.len());

        for row in result_set.into_iter() {
            views.push(View {
                name: row.get_expect_string("view_name"),
                definition: row.get_expect_string("view_sql"),
            })
        }

        Ok(views)
    }

    #[tracing::instrument]
    async fn get_columns(&self, table: &str) -> DescriberResult<(Vec<Column>, Option<PrimaryKey>)> {
        let sql = format!(r#"PRAGMA table_info ("{}")"#, table);
        let result_set = self.conn.query_raw(&sql, &[]).await?;
        let mut pk_cols: HashMap<i64, String> = HashMap::new();
        let mut cols: Vec<Column> = result_set
            .into_iter()
            .map(|row| {
                trace!("Got column row {:?}", row);
                let is_required = row.get("notnull").and_then(|x| x.as_bool()).expect("notnull");

                let arity = if is_required {
                    ColumnArity::Required
                } else {
                    ColumnArity::Nullable
                };
                let tpe = get_column_type(&row.get("type").and_then(|x| x.to_string()).expect("type"), arity);

                let default = match row.get("dflt_value") {
                    None => None,
                    Some(val) if val.is_null() => None,
                    Some(Value::Text(Some(cow_string))) => {
                        let default_string = cow_string.to_string();

                        if default_string.to_lowercase() == "null" {
                            None
                        } else {
                            Some(match &tpe.family {
                                ColumnTypeFamily::Int => match Self::parse_int(&default_string) {
                                    Some(int_value) => DefaultValue::value(int_value),
                                    None => DefaultValue::db_generated(default_string),
                                },
                                ColumnTypeFamily::BigInt => match Self::parse_big_int(&default_string) {
                                    Some(int_value) => DefaultValue::value(int_value),
                                    None => DefaultValue::db_generated(default_string),
                                },
                                ColumnTypeFamily::Float => match Self::parse_float(&default_string) {
                                    Some(float_value) => DefaultValue::value(float_value),
                                    None => DefaultValue::db_generated(default_string),
                                },
                                ColumnTypeFamily::Decimal => match Self::parse_float(&default_string) {
                                    Some(float_value) => DefaultValue::value(float_value),
                                    None => DefaultValue::db_generated(default_string),
                                },
                                ColumnTypeFamily::Boolean => match Self::parse_int(&default_string) {
                                    Some(PrismaValue::Int(1)) => DefaultValue::value(true),
                                    Some(PrismaValue::Int(0)) => DefaultValue::value(false),
                                    _ => match Self::parse_bool(&default_string) {
                                        Some(bool_value) => DefaultValue::value(bool_value),
                                        None => DefaultValue::db_generated(default_string),
                                    },
                                },
                                ColumnTypeFamily::String => {
                                    DefaultValue::value(unquote_sqlite_string_default(&default_string).into_owned())
                                }
                                ColumnTypeFamily::DateTime => match default_string.to_lowercase().as_str() {
                                    "current_timestamp" | "datetime(\'now\')" | "datetime(\'now\', \'localtime\')" => {
                                        DefaultValue::now()
                                    }
                                    _ => DefaultValue::db_generated(default_string),
                                },
                                ColumnTypeFamily::Binary => DefaultValue::db_generated(default_string),
                                ColumnTypeFamily::Json => DefaultValue::db_generated(default_string),
                                ColumnTypeFamily::Uuid => DefaultValue::db_generated(default_string),
                                ColumnTypeFamily::Enum(_) => DefaultValue::value(PrismaValue::Enum(default_string)),
                                ColumnTypeFamily::Unsupported(_) => DefaultValue::db_generated(default_string),
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

                trace!(
                    "Found column '{}', type: '{:?}', default: {:?}, primary key: {}",
                    col.name,
                    col.tpe,
                    col.default,
                    pk_col > 0
                );

                col
            })
            .collect();

        let primary_key = if pk_cols.is_empty() {
            trace!("Determined that table has no primary key");
            None
        } else {
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
                    if &col.name == pk_col && &col.tpe.full_data_type.to_lowercase() == "integer" {
                        trace!(
                            "Detected that the primary key column corresponds to rowid and \
                                 is auto incrementing"
                        );
                        col.auto_increment = true;
                    }
                }
            }

            trace!("Determined that table has primary key with columns {:?}", columns);
            Some(PrimaryKey {
                columns,
                sequence: None,
                constraint_name: None,
            })
        };

        Ok((cols, primary_key))
    }

    async fn get_foreign_keys(&self, table: &str) -> DescriberResult<Vec<ForeignKey>> {
        struct IntermediateForeignKey {
            pub columns: HashMap<i64, String>,
            pub referenced_table: String,
            pub referenced_columns: HashMap<i64, String>,
            pub on_delete_action: ForeignKeyAction,
            pub on_update_action: ForeignKeyAction,
        }

        let sql = format!(r#"PRAGMA foreign_key_list("{}");"#, table);
        trace!("describing table foreign keys, SQL: '{}'", sql);
        let result_set = self.conn.query_raw(&sql, &[]).await.expect("querying for foreign keys");

        // Since one foreign key with multiple columns will be represented here as several
        // rows with the same ID, we have to use an intermediate representation that gets
        // translated into the real foreign keys in another pass
        let mut intermediate_fks: HashMap<i64, IntermediateForeignKey> = HashMap::new();
        for row in result_set.into_iter() {
            trace!("got FK description row {:?}", row);
            let id = row.get("id").and_then(|x| x.as_i64()).expect("id");
            let seq = row.get("seq").and_then(|x| x.as_i64()).expect("seq");
            let column = row.get("from").and_then(|x| x.to_string()).expect("from");
            // this can be null if the primary key and shortened fk syntax was used
            let referenced_column = row.get("to").and_then(|x| x.to_string());
            let referenced_table = row.get("table").and_then(|x| x.to_string()).expect("table");
            match intermediate_fks.get_mut(&id) {
                Some(fk) => {
                    fk.columns.insert(seq, column);
                    if let Some(column) = referenced_column {
                        fk.referenced_columns.insert(seq, column);
                    };
                }
                None => {
                    let mut columns: HashMap<i64, String> = HashMap::new();
                    columns.insert(seq, column);
                    let mut referenced_columns: HashMap<i64, String> = HashMap::new();

                    if let Some(column) = referenced_column {
                        referenced_columns.insert(seq, column);
                    };
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
                        s => panic!(format!("Unrecognized on delete action '{}'", s)),
                    };
                    let on_update_action = match row
                        .get("on_update")
                        .and_then(|x| x.to_string())
                        .expect("on_update")
                        .to_lowercase()
                        .as_str()
                    {
                        "no action" => ForeignKeyAction::NoAction,
                        "restrict" => ForeignKeyAction::Restrict,
                        "set null" => ForeignKeyAction::SetNull,
                        "set default" => ForeignKeyAction::SetDefault,
                        "cascade" => ForeignKeyAction::Cascade,
                        s => panic!(format!("Unrecognized on update action '{}'", s)),
                    };
                    let fk = IntermediateForeignKey {
                        columns,
                        referenced_table,
                        referenced_columns,
                        on_delete_action,
                        on_update_action,
                    };
                    intermediate_fks.insert(id, fk);
                }
            };
        }

        let mut fks: Vec<ForeignKey> = intermediate_fks
            .values()
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
                    on_update_action: intermediate_fk.on_update_action.to_owned(),

                    // Not relevant in SQLite since we cannot ALTER or DROP foreign keys by
                    // constraint name.
                    constraint_name: None,
                };

                trace!("Detected foreign key {:?}", fk);

                fk
            })
            .collect();

        fks.sort_unstable_by_key(|fk| fk.columns.clone());

        Ok(fks)
    }

    async fn get_indices(&self, table: &str, columns: &[Column]) -> DescriberResult<Vec<Index>> {
        let sql = format!(r#"PRAGMA index_list("{}");"#, table);
        let result_set = self.conn.query_raw(&sql, &[]).await?;
        trace!("Got indices description results: {:?}", result_set);

        let mut indices = Vec::new();
        let filtered_rows = result_set
            .into_iter()
            // Exclude primary keys, they are inferred separately.
            .filter(|row| row.get("origin").and_then(|origin| origin.as_str()).unwrap() != "pk")
            // Exclude partial indices
            .filter(|row| !row.get("partial").and_then(|partial| partial.as_bool()).unwrap());

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

            let sql = format!(r#"PRAGMA index_info("{}");"#, name);
            let result_set = self.conn.query_raw(&sql, &[]).await.expect("querying for index info");
            trace!("Got index description results: {:?}", result_set);
            for row in result_set.into_iter() {
                let pos = row.get("seqno").and_then(|x| x.as_i64()).expect("get seqno") as usize;
                let col_name = row.get("name").and_then(|x| x.to_string()).expect("get name");
                let column_idx_in_table = columns
                    .iter()
                    .position(|col| col.name == col_name)
                    .ok_or_else(|| format!("Index {} refers to unknown column {}", name, col_name))
                    .unwrap();

                if index.columns.len() <= pos {
                    index.columns.resize(pos + 1, 0);
                }

                index.columns[pos] = column_idx_in_table;
            }

            indices.push(index)
        }

        Ok(indices)
    }
}

fn get_column_type(tpe: &str, arity: ColumnArity) -> ColumnType {
    let tpe_lower = tpe.to_lowercase();

    let family = match tpe_lower.as_ref() {
        // SQLite only has a few native data types: https://www.sqlite.org/datatype3.html
        // It's tolerant though, and you can assign any data type you like to columns
        "int" => ColumnTypeFamily::Int,
        "integer" => ColumnTypeFamily::Int,
        "bigint" => ColumnTypeFamily::BigInt,
        "real" => ColumnTypeFamily::Float,
        "float" => ColumnTypeFamily::Float,
        "serial" => ColumnTypeFamily::Int,
        "boolean" => ColumnTypeFamily::Boolean,
        "text" => ColumnTypeFamily::String,
        s if s.contains("char") => ColumnTypeFamily::String,
        s if s.contains("numeric") => ColumnTypeFamily::Decimal,
        s if s.contains("decimal") => ColumnTypeFamily::Decimal,
        "date" => ColumnTypeFamily::DateTime,
        "datetime" => ColumnTypeFamily::DateTime,
        "timestamp" => ColumnTypeFamily::DateTime,
        "binary" | "blob" => ColumnTypeFamily::Binary,
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
        // NUMERIC type affinity
        data_type if data_type.starts_with("decimal") => ColumnTypeFamily::Decimal,
        data_type => ColumnTypeFamily::Unsupported(data_type.into()),
    };
    ColumnType {
        full_data_type: tpe.to_string(),
        family,
        arity,
        native_type: None,
    }
}

// "A string constant is formed by enclosing the string in single quotes ('). A single quote within
// the string can be encoded by putting two single quotes in a row - as in Pascal. C-style escapes
// using the backslash character are not supported because they are not standard SQL."
//
// - https://www.sqlite.org/lang_expr.html
fn unquote_sqlite_string_default(s: &str) -> Cow<'_, str> {
    static SQLITE_STRING_DEFAULT_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r#"(?ms)^'(.*)'$|^"(.*)"$"#).unwrap());
    static SQLITE_ESCAPED_CHARACTER_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r#"''"#).unwrap());

    match SQLITE_STRING_DEFAULT_RE.replace(s, "$1$2") {
        Cow::Borrowed(s) => SQLITE_ESCAPED_CHARACTER_RE.replace_all(s, "'"),
        Cow::Owned(s) => SQLITE_ESCAPED_CHARACTER_RE.replace_all(&s, "'").into_owned().into(),
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
