//! SQLite description.
#![cfg_attr(target_arch = "wasm32", allow(dead_code))]

use crate::{
    getters::Getter, ids::*, parsers::Parser, Column, ColumnArity, ColumnType, ColumnTypeFamily, DefaultValue,
    DescriberResult, ForeignKeyAction, PrismaValue, Regex, SQLSortOrder, SqlSchema,
};
use either::Either;
use indexmap::IndexMap;
use quaint::{
    ast::{Value, ValueType},
    connector::AdapterName,
    prelude::{Queryable, ResultRow},
};
use std::{
    any::type_name,
    borrow::Cow,
    collections::BTreeMap,
    fmt::Debug,
    path::Path,
    sync::{Arc, LazyLock, OnceLock},
};
use tracing::trace;

#[cfg(feature = "sqlite-native")]
pub(crate) mod native;

#[async_trait::async_trait]
pub trait Connection {
    async fn query_raw<'a>(
        &'a self,
        sql: &'a str,
        params: &'a [quaint::prelude::Value<'a>],
    ) -> quaint::Result<quaint::prelude::ResultSet>;

    fn adapter_name(&self) -> Option<AdapterName> {
        None
    }
}

#[async_trait::async_trait]
impl Connection for quaint::single::Quaint {
    async fn query_raw<'a>(
        &'a self,
        sql: &'a str,
        params: &'a [quaint::prelude::Value<'a>],
    ) -> quaint::Result<quaint::prelude::ResultSet> {
        quaint::prelude::Queryable::query_raw(self, sql, params).await
    }
}

#[async_trait::async_trait]
impl<Q: Queryable + ?Sized> Connection for Arc<Q> {
    async fn query_raw<'a>(
        &'a self,
        sql: &'a str,
        params: &'a [quaint::prelude::Value<'a>],
    ) -> quaint::Result<quaint::prelude::ResultSet> {
        quaint::prelude::Queryable::query_raw(&**self, sql, params).await
    }

    fn adapter_name(&self) -> Option<AdapterName> {
        self.as_external_connector().map(|adapter| adapter.adapter_name())
    }
}

pub struct SqlSchemaDescriber<'a> {
    conn: &'a (dyn Connection + Send + Sync),
}

impl Debug for SqlSchemaDescriber<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct(type_name::<SqlSchemaDescriber<'_>>()).finish()
    }
}

impl Parser for SqlSchemaDescriber<'_> {}

impl<'a> SqlSchemaDescriber<'a> {
    /// Constructor.
    pub fn new(conn: &'a (dyn Connection + Send + Sync)) -> SqlSchemaDescriber<'a> {
        SqlSchemaDescriber { conn }
    }

    pub async fn describe_impl(&self) -> DescriberResult<SqlSchema> {
        let mut schema = SqlSchema::default();
        let container_ids = self.get_table_names(&mut schema).await?;
        let table_ids: IndexMap<&str, TableId> = container_ids
            .iter()
            .filter_map(|(name, id)| id.left().map(|id| (name.as_str(), id)))
            .collect();

        for (container_name, container_id) in &container_ids {
            push_columns(container_name, *container_id, &mut schema, self.conn).await?;

            if let Either::Left(table_id) = container_id {
                push_indexes(container_name, *table_id, &mut schema, self.conn).await?;
            }
        }

        for (table_name, table_id) in &table_ids {
            self.push_foreign_keys(table_name, *table_id, &table_ids, &mut schema)
                .await?;
        }

        Ok(schema)
    }

    async fn get_databases(&self) -> DescriberResult<Vec<String>> {
        let sql = "PRAGMA database_list;";
        let rows = self.conn.query_raw(sql, &[]).await?;
        let names = rows
            .into_iter()
            .map(|row| {
                row.get("file")
                    .and_then(|x| x.to_string())
                    .and_then(|x| {
                        Path::new(&x)
                            .file_name()
                            .map(|name| name.to_string_lossy().into_owned())
                    })
                    .expect("convert schema names")
            })
            .collect();

        trace!("Found schema names: {:?}", names);

        Ok(names)
    }

    pub async fn get_table_names(
        &self,
        schema: &mut SqlSchema,
    ) -> DescriberResult<IndexMap<String, Either<TableId, ViewId>>> {
        let sql = r#"SELECT name, type, sql FROM sqlite_master WHERE type='table' OR type='view' ORDER BY name ASC"#;

        let result_set = self.conn.query_raw(sql, &[]).await?;

        let names = result_set
            .into_iter()
            .map(|row| {
                let r#type = row.get("type").and_then(|x| x.to_string()).unwrap();
                let name = row.get("name").and_then(|x| x.to_string()).unwrap();
                let definition = row.get("sql").and_then(|x| x.to_string());

                (name, r#type, definition)
            })
            .filter(|(table_name, _, _)| !is_table_ignored(table_name, self.conn.adapter_name()));

        let mut map = IndexMap::default();

        for (name, r#type, definition) in names {
            let cloned_name = name.clone();

            match r#type.as_str() {
                "table" => {
                    let id = schema.push_table(name, Default::default(), None);
                    map.insert(cloned_name, Either::Left(id));
                }
                "view" => {
                    let id = schema.push_view(name, Default::default(), definition, None);
                    map.insert(cloned_name, Either::Right(id));
                }
                _ => unreachable!(),
            }
        }

        Ok(map)
    }

    async fn push_foreign_keys(
        &self,
        table_name: &str,
        table_id: TableId,
        table_ids: &IndexMap<&str, TableId>,
        schema: &mut SqlSchema,
    ) -> DescriberResult<()> {
        let sql = format!(r#"PRAGMA foreign_key_list("{table_name}");"#);
        let result_set = self.conn.query_raw(&sql, &[]).await?;
        let mut current_foreign_key: Option<(i64, ForeignKeyId)> = None;
        let mut current_foreign_key_columns: Vec<(i64, TableColumnId, Option<TableColumnId>)> = Vec::new();

        fn get_ids(
            row: &ResultRow,
            table_id: TableId,
            table_ids: &IndexMap<&str, TableId>,
            schema: &SqlSchema,
        ) -> Option<(TableColumnId, TableId, Option<TableColumnId>)> {
            let column = schema.walk(table_id).column(&row.get_expect_string("from"))?.id;
            let referenced_table = schema.walk(*table_ids.get(row.get_expect_string("table").as_str())?);
            // this can be null if the primary key and shortened fk syntax was used
            let referenced_column = row
                .get_string("to")
                .and_then(|colname| Some(referenced_table.column(&colname)?.id));

            Some((column, referenced_table.id, referenced_column))
        }

        fn get_referential_actions(row: &ResultRow) -> [ForeignKeyAction; 2] {
            let on_delete_action = match row.get_expect_string("on_delete").to_lowercase().as_str() {
                "no action" => ForeignKeyAction::NoAction,
                "restrict" => ForeignKeyAction::Restrict,
                "set null" => ForeignKeyAction::SetNull,
                "set default" => ForeignKeyAction::SetDefault,
                "cascade" => ForeignKeyAction::Cascade,
                s => panic!("Unrecognized on delete action '{s}'"),
            };
            let on_update_action = match row.get_expect_string("on_update").to_lowercase().as_str() {
                "no action" => ForeignKeyAction::NoAction,
                "restrict" => ForeignKeyAction::Restrict,
                "set null" => ForeignKeyAction::SetNull,
                "set default" => ForeignKeyAction::SetDefault,
                "cascade" => ForeignKeyAction::Cascade,
                s => panic!("Unrecognized on update action '{s}'"),
            };
            [on_delete_action, on_update_action]
        }

        fn flush_current_fk(
            current_foreign_key: &mut Option<(i64, ForeignKeyId)>,
            current_columns: &mut Vec<(i64, TableColumnId, Option<TableColumnId>)>,
            schema: &mut SqlSchema,
        ) {
            current_columns.sort_by_key(|(seq, _, _)| *seq);
            let fkid = if let Some((_, id)) = current_foreign_key {
                *id
            } else {
                return;
            };

            // SQLite allows foreign key definitions without specifying the referenced columns, it then
            // assumes the pk is used.
            if current_columns[0].2.is_none() {
                let referenced_table_pk_columns = schema
                    .walk(fkid)
                    .referenced_table()
                    .primary_key_columns()
                    .into_iter()
                    .flatten()
                    .map(|w| w.as_column().id)
                    .collect::<Vec<_>>();

                if referenced_table_pk_columns.len() == current_columns.len() {
                    for (col, referenced) in current_columns.drain(..).zip(referenced_table_pk_columns) {
                        schema.push_foreign_key_column(fkid, [col.1, referenced]);
                    }
                }
            } else {
                for (_, col, referenced) in current_columns.iter() {
                    schema.push_foreign_key_column(fkid, [*col, referenced.unwrap()]);
                }
            }

            *current_foreign_key = None;
            current_columns.clear();
        }

        for row in result_set.into_iter() {
            trace!("got FK description row {:?}", row);
            let id = row.get("id").and_then(|x| x.as_integer()).expect("id");
            let seq = row.get("seq").and_then(|x| x.as_integer()).expect("seq");
            let (column_id, referenced_table_id, referenced_column_id) =
                if let Some(ids) = get_ids(&row, table_id, table_ids, schema) {
                    ids
                } else {
                    continue;
                };

            match &mut current_foreign_key {
                None => {
                    let foreign_key_id =
                        schema.push_foreign_key(None, [table_id, referenced_table_id], get_referential_actions(&row));
                    current_foreign_key = Some((id, foreign_key_id));
                }
                Some((sqlite_id, _)) if *sqlite_id == id => {}
                Some(_) => {
                    // Flush current foreign key.
                    flush_current_fk(&mut current_foreign_key, &mut current_foreign_key_columns, schema);

                    let foreign_key_id =
                        schema.push_foreign_key(None, [table_id, referenced_table_id], get_referential_actions(&row));
                    current_foreign_key = Some((id, foreign_key_id));
                }
            }

            current_foreign_key_columns.push((seq, column_id, referenced_column_id));
        }

        // Flush the last foreign key.
        flush_current_fk(&mut current_foreign_key, &mut current_foreign_key_columns, schema);

        Ok(())
    }
}

async fn push_columns(
    table_name: &str,
    container_id: Either<TableId, ViewId>,
    schema: &mut SqlSchema,
    conn: &(dyn Connection + Send + Sync),
) -> DescriberResult<()> {
    let sql = format!(r#"PRAGMA table_info ("{table_name}")"#);
    let result_set = conn.query_raw(&sql, &[]).await?;
    let mut pk_cols: BTreeMap<i64, TableColumnId> = BTreeMap::new();
    for row in result_set {
        trace!("Got column row {row:?}");
        let is_required = row.get("notnull").and_then(|x| x.as_bool()).expect("notnull");

        let arity = if is_required {
            ColumnArity::Required
        } else {
            ColumnArity::Nullable
        };

        let tpe = get_column_type(row.get_expect_string("type"), arity);

        let default = match row.get("dflt_value") {
            None => None,
            Some(val) if val.is_null() => None,
            Some(Value {
                typed: ValueType::Text(Some(cow_string)),
                ..
            }) => {
                let default_string = cow_string.to_string();

                if default_string.to_lowercase() == "null" {
                    None
                } else {
                    Some(match &tpe.family {
                        ColumnTypeFamily::Int => match SqlSchemaDescriber::parse_int(&default_string) {
                            Some(int_value) => DefaultValue::value(int_value),
                            None => DefaultValue::db_generated(default_string),
                        },
                        ColumnTypeFamily::BigInt => match SqlSchemaDescriber::parse_big_int(&default_string) {
                            Some(int_value) => DefaultValue::value(int_value),
                            None => DefaultValue::db_generated(default_string),
                        },
                        ColumnTypeFamily::Float => match SqlSchemaDescriber::parse_float(&default_string) {
                            Some(float_value) => DefaultValue::value(float_value),
                            None => DefaultValue::db_generated(default_string),
                        },
                        ColumnTypeFamily::Decimal => match SqlSchemaDescriber::parse_float(&default_string) {
                            Some(float_value) => DefaultValue::value(float_value),
                            None => DefaultValue::db_generated(default_string),
                        },
                        ColumnTypeFamily::Boolean => match SqlSchemaDescriber::parse_int(&default_string) {
                            Some(PrismaValue::Int(1)) => DefaultValue::value(true),
                            Some(PrismaValue::Int(0)) => DefaultValue::value(false),
                            _ => match SqlSchemaDescriber::parse_bool(&default_string) {
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
                        ColumnTypeFamily::Json => DefaultValue::value(default_string),
                        ColumnTypeFamily::Binary => DefaultValue::db_generated(default_string),
                        ColumnTypeFamily::Uuid => DefaultValue::db_generated(default_string),
                        ColumnTypeFamily::Enum(_) => DefaultValue::value(PrismaValue::Enum(default_string)),
                        ColumnTypeFamily::Unsupported(_) => DefaultValue::db_generated(default_string),
                    })
                }
            }
            Some(_) => None,
        };

        let column = Column {
            name: row.get_expect_string("name"),
            tpe,
            auto_increment: false,
            description: None,
        };

        match container_id {
            Either::Left(table_id) => {
                let pk_col = row.get("pk").and_then(|x| x.as_integer()).expect("primary key");
                let column_id = schema.push_table_column(table_id, column);

                if pk_col > 0 {
                    pk_cols.insert(pk_col, column_id);
                }

                if let Some(default) = default {
                    schema.push_table_default_value(column_id, default);
                }
            }
            Either::Right(view_id) => {
                let column_id = schema.push_view_column(view_id, column);

                if let Some(default) = default {
                    schema.push_view_default_value(column_id, default);
                }
            }
        }
    }

    if let Either::Left(table_id) = container_id {
        if !pk_cols.is_empty() {
            let pk_id = schema.push_primary_key(table_id, String::new());
            for column_id in pk_cols.values() {
                schema.push_index_column(crate::IndexColumn {
                    index_id: pk_id,
                    column_id: *column_id,
                    sort_order: None,
                    length: None,
                });
            }

            // Integer ID columns are always implemented with either row id or autoincrement
            if pk_cols.len() == 1 {
                let pk_col_id = *pk_cols.values().next().unwrap();
                let pk_col = &mut schema.table_columns[pk_col_id.0 as usize];
                // See https://www.sqlite.org/lang_createtable.html for the exact logic.
                if pk_col.1.tpe.full_data_type.eq_ignore_ascii_case("INTEGER") {
                    pk_col.1.auto_increment = true;
                    pk_col.1.tpe.arity = ColumnArity::Required;
                }
            }
        }
    }

    schema.table_default_values.sort_by_key(|(column_id, _)| *column_id);
    schema.view_default_values.sort_by_key(|(column_id, _)| *column_id);

    Ok(())
}

async fn push_indexes(
    table: &str,
    table_id: TableId,
    schema: &mut SqlSchema,
    conn: &(dyn Connection + Send + Sync),
) -> DescriberResult<()> {
    let sql = format!(r#"PRAGMA index_list("{table}");"#);
    let result_set = conn.query_raw(&sql, &[]).await?;
    let mut indexes = Vec::new(); // (index_name, is_unique, columns)

    let filtered_rows = result_set
        .into_iter()
        // Exclude primary keys, they are inferred separately.
        .filter(|row| row.get("origin").and_then(|origin| origin.as_str()).unwrap() != "pk")
        // Exclude partial indices
        .filter(|row| !row.get("partial").and_then(|partial| partial.as_bool()).unwrap());

    for row in filtered_rows {
        let mut valid_index = true;

        let is_unique = row.get_expect_bool("unique");
        let index_name = row.get_expect_string("name");
        let mut columns = Vec::new();

        let sql = format!(r#"PRAGMA index_info("{index_name}");"#);
        let result_set = conn.query_raw(&sql, &[]).await?;
        trace!("Got index description results: {result_set:?}");

        for row in result_set.into_iter() {
            // if the index is on a rowid or expression, the name of the column will be null,
            // we ignore these for now
            match row
                .get_string("name")
                .and_then(|name| schema.walk(table_id).column(&name))
            {
                Some(col) => {
                    columns.push((col.id, SQLSortOrder::Asc));
                }
                None => valid_index = false,
            }
        }

        let sql = format!(r#"PRAGMA index_xinfo("{index_name}");"#);
        let result_set = conn.query_raw(&sql, &[]).await?;
        trace!("Got index description results: {result_set:?}");

        for row in result_set.into_iter() {
            //if the index is on a rowid or expression, the name of the column will be null, we ignore these for now
            if row.get("name").and_then(|x| x.to_string()).is_some() {
                let pos = row.get_expect_i64("seqno");
                let sort_order = match row.get_expect_i64("desc") {
                    0 => SQLSortOrder::Asc,
                    _ => SQLSortOrder::Desc,
                };
                if let Some(col) = columns.get_mut(pos as usize) {
                    col.1 = sort_order;
                }
            }
        }

        if valid_index {
            indexes.push((index_name, is_unique, columns))
        }
    }

    for (index_name, unique, columns) in indexes {
        let index_id = if unique {
            schema.push_unique_constraint(table_id, index_name)
        } else {
            schema.push_index(table_id, index_name)
        };

        for (column_id, sort_order) in columns {
            schema.push_index_column(crate::IndexColumn {
                index_id,
                column_id,
                sort_order: Some(sort_order),
                length: None,
            });
        }
    }

    Ok(())
}

fn get_column_type(mut tpe: String, arity: ColumnArity) -> ColumnType {
    tpe.make_ascii_lowercase();
    let family = match tpe.as_ref() {
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
        "jsonb" => ColumnTypeFamily::Json,
        // NUMERIC type affinity
        data_type if data_type.starts_with("decimal") => ColumnTypeFamily::Decimal,
        data_type => ColumnTypeFamily::Unsupported(data_type.into()),
    };
    ColumnType {
        full_data_type: tpe,
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
    static SQLITE_STRING_DEFAULT_RE: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r#"(?ms)^'(.*)'$|^"(.*)"$"#).unwrap());
    static SQLITE_ESCAPED_CHARACTER_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r#"''"#).unwrap());

    match SQLITE_STRING_DEFAULT_RE.replace(s, "$1$2") {
        Cow::Borrowed(s) => SQLITE_ESCAPED_CHARACTER_RE.replace_all(s, "'"),
        Cow::Owned(s) => SQLITE_ESCAPED_CHARACTER_RE.replace_all(&s, "'").into_owned().into(),
    }
}

// Create a OnceLock to hold the compiled Regex
static CLOUDFLARE_D1_IGNORED_TABLES_REGEX: OnceLock<Regex> = OnceLock::new();

// Cloudflare D1 specific tables, excludes `_cf_KV`, `_cf_METADATA`, etc.
fn get_cloudflare_d1_ignored_tables_regex() -> &'static Regex {
    CLOUDFLARE_D1_IGNORED_TABLES_REGEX.get_or_init(|| Regex::new(r"^(_cf_[A-Z]+).*$").expect("Failed to compile regex"))
}

/// Returns whether a table is one of the SQLite system tables or a Cloudflare D1 specific table.
fn is_table_ignored(table_name: &str, _adapter_name: Option<AdapterName>) -> bool {
    let early_result = SQLITE_IGNORED_TABLES.contains(&table_name);

    // TODO: remove the constant `is_cloudflare_d1 = true` and replace it with the following once we
    // get rid of `--local-d1`, `--to-local-d1`, `--from-local-d1` flags in the CLI.
    // ```
    // let is_cloudflare_d1 = matches!(adapter_name, Some(AdapterName::D1(_)));
    // ```
    let is_cloudflare_d1 = true;

    if is_cloudflare_d1 {
        early_result || get_cloudflare_d1_ignored_tables_regex().is_match(table_name)
    } else {
        early_result
    }
}

/// See https://www.sqlite.org/fileformat2.html
/// + Cloudflare D1 specific tables
const SQLITE_IGNORED_TABLES: &[&str] = &[
    // SQLite system tables
    "sqlite_sequence",
    "sqlite_stat1",
    "sqlite_stat2",
    "sqlite_stat3",
    "sqlite_stat4",
    // This is the default but can be configured by the user
    "d1_migrations",
];
