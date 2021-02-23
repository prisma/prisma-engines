//! Postgres schema description.

use super::*;
use crate::{getters::Getter, parsers::Parser};
use indoc::indoc;
use native_types::{NativeType, PostgresType};
use quaint::{connector::ResultRow, prelude::Queryable, single::Quaint};
use regex::Regex;
use serde_json::from_str;
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
        let size_in_bytes = self.get_size(&schema).await?;

        Ok(SQLMetadata {
            table_count,
            size_in_bytes,
        })
    }

    #[tracing::instrument]
    async fn describe(&self, schema: &str) -> DescriberResult<SqlSchema> {
        let sequences = self.get_sequences(schema).await?;
        let enums = self.get_enums(schema).await?;
        let mut columns = self.get_columns(schema, &enums, &sequences).await?;
        let mut foreign_keys = self.get_foreign_keys(schema).await?;
        let mut indexes = self.get_indices(schema, &sequences, &columns).await?;

        let table_names = self.get_table_names(schema).await?;
        let mut tables = Vec::with_capacity(table_names.len());

        for table_name in &table_names {
            tables.push(self.get_table(&table_name, &mut columns, &mut foreign_keys, &mut indexes));
        }

        let views = self.get_views(schema).await?;
        let procedures = self.get_procedures(schema).await?;

        Ok(SqlSchema {
            enums,
            sequences,
            tables,
            views,
            procedures,
        })
    }

    #[tracing::instrument]
    async fn version(&self, schema: &str) -> crate::DescriberResult<Option<String>> {
        Ok(self.conn.version().await?)
    }
}

static PG_RE_NUM: Lazy<Regex> = Lazy::new(|| Regex::new(r"^'?(-?\d+)('::.*)?$").expect("compile regex"));
static PG_RE_FLOAT: Lazy<Regex> = Lazy::new(|| Regex::new(r"^'?([^']+)('::.*)?$").expect("compile regex"));

impl Parser for SqlSchemaDescriber {
    fn re_num() -> &'static Regex {
        &PG_RE_NUM
    }

    fn re_float() -> &'static Regex {
        &PG_RE_FLOAT
    }
}

impl SqlSchemaDescriber {
    /// Constructor.
    pub fn new(conn: Quaint) -> SqlSchemaDescriber {
        SqlSchemaDescriber { conn }
    }

    #[tracing::instrument]
    async fn get_databases(&self) -> DescriberResult<Vec<String>> {
        let sql = "select schema_name from information_schema.schemata;";
        let rows = self.conn.query_raw(sql, &[]).await?;
        let names = rows
            .into_iter()
            .map(|row| row.get_expect_string("schema_name"))
            .collect();

        trace!("Found schema names: {:?}", names);

        Ok(names)
    }

    #[tracing::instrument]
    async fn get_procedures(&self, schema: &str) -> DescriberResult<Vec<Procedure>> {
        let sql = r#"
            SELECT p.proname AS name,
                CASE WHEN l.lanname = 'internal' THEN p.prosrc
                     ELSE pg_get_functiondef(p.oid)
                     END as definition
            FROM pg_proc p
            LEFT JOIN pg_namespace n ON p.pronamespace = n.oid
            LEFT JOIN pg_language l ON p.prolang = l.oid
            WHERE n.nspname = $1
        "#;

        let rows = self.conn.query_raw(sql, &[schema.into()]).await?;
        let mut procedures = Vec::with_capacity(rows.len());

        for row in rows.into_iter() {
            procedures.push(Procedure {
                name: row.get_expect_string("name"),
                definition: row.get_expect_string("definition"),
            });
        }

        Ok(procedures)
    }

    #[tracing::instrument]
    async fn get_table_names(&self, schema: &str) -> DescriberResult<Vec<String>> {
        let sql = "
            SELECT table_name as table_name FROM information_schema.tables
            WHERE table_schema = $1
            -- Views are not supported yet
            AND table_type = 'BASE TABLE'
            ORDER BY table_name";
        let rows = self.conn.query_raw(sql, &[schema.into()]).await?;
        let names = rows
            .into_iter()
            .map(|row| row.get_expect_string("table_name"))
            .collect();

        trace!("Found table names: {:?}", names);

        Ok(names)
    }

    #[tracing::instrument]
    async fn get_size(&self, schema: &str) -> DescriberResult<usize> {
        let sql =
            "SELECT SUM(pg_total_relation_size(quote_ident(schemaname) || '.' || quote_ident(tablename)))::BIGINT as size
             FROM pg_tables
             WHERE schemaname = $1::text";
        let mut result_iter = self.conn.query_raw(sql, &[schema.into()]).await?.into_iter();
        let size: i64 = result_iter.next().and_then(|row| row.get_i64("size")).unwrap_or(0);

        trace!("Found db size: {:?}", size);

        Ok(size.try_into().expect("size is not a valid usize"))
    }

    #[tracing::instrument(skip(columns, foreign_keys, indices))]
    fn get_table(
        &self,
        name: &str,
        columns: &mut HashMap<String, Vec<Column>>,
        foreign_keys: &mut HashMap<String, Vec<ForeignKey>>,
        indices: &mut HashMap<String, (Vec<Index>, Option<PrimaryKey>)>,
    ) -> Table {
        let (indices, primary_key) = indices.remove(name).unwrap_or_else(|| (Vec::new(), None));
        let foreign_keys = foreign_keys.remove(name).unwrap_or_else(Vec::new);
        let columns = columns.remove(name).unwrap_or_default();
        Table {
            name: name.to_string(),
            columns,
            foreign_keys,
            indices,
            primary_key,
        }
    }

    #[tracing::instrument]
    async fn get_views(&self, schema: &str) -> DescriberResult<Vec<View>> {
        let sql = indoc! {r#"
            SELECT viewname AS view_name, definition AS view_sql
            FROM pg_catalog.pg_views
            WHERE schemaname = $1
        "#};

        let result_set = self.conn.query_raw(sql, &[schema.into()]).await?;
        let mut views = Vec::with_capacity(result_set.len());

        for row in result_set.into_iter() {
            views.push(View {
                name: row.get_expect_string("view_name"),
                definition: row.get_expect_string("view_sql"),
            })
        }

        Ok(views)
    }

    async fn get_columns(
        &self,
        schema: &str,
        enums: &[Enum],
        sequences: &[Sequence],
    ) -> DescriberResult<HashMap<String, Vec<Column>>> {
        let mut columns: HashMap<String, Vec<Column>> = HashMap::new();

        let sql = r#"
            SELECT
               info.table_name,
                info.column_name,
                format_type(att.atttypid, att.atttypmod) as formatted_type,
                info.numeric_precision,
                info.numeric_scale,
                info.numeric_precision_radix,
                info.datetime_precision,
                info.data_type,
                info.udt_name as full_data_type,
                info.column_default,
                info.is_nullable,
                info.is_identity,
                info.data_type,
                info.character_maximum_length
            FROM information_schema.columns info
            JOIN pg_attribute  att on att.attname = info.column_name
            And att.attrelid = (
            	SELECT pg_class.oid
            	FROM pg_class
            	JOIN pg_namespace on pg_namespace.oid = pg_class.relnamespace
            	WHERE relname = info.table_name
            	AND pg_namespace.nspname = $1
            	)
            WHERE table_schema = $1
            ORDER BY ordinal_position;
        "#;

        let rows = self.conn.query_raw(&sql, &[schema.into()]).await?;

        for col in rows {
            trace!("Got column: {:?}", col);
            let table_name = col.get_expect_string("table_name");
            let name = col.get_expect_string("column_name");

            let is_identity_str = col.get_expect_string("is_identity").to_lowercase();

            let is_identity = match is_identity_str.as_str() {
                "no" => false,
                "yes" => true,
                _ => panic!("unrecognized is_identity variant '{}'", is_identity_str),
            };

            let tpe = get_column_type(&col, enums);
            let data_type = col.get_expect_string("data_type");

            let default = Self::get_default_value(&col, &tpe, &data_type, sequences);

            let auto_increment =
                is_identity || matches!(default.as_ref().map(|d| d.kind()), Some(DefaultKind::SEQUENCE(_)));

            let col = Column {
                name,
                tpe,
                default,
                auto_increment,
            };

            columns.entry(table_name).or_default().push(col);
        }

        trace!("Found table columns: {:?}", columns);

        Ok(columns)
    }

    fn get_precision(col: &ResultRow) -> Precision {
        let (character_maximum_length, numeric_precision, numeric_scale, time_precision) =
            if matches!(col.get_expect_string("data_type").as_str(), "ARRAY") {
                fn get_single(formatted_type: &str) -> Option<u32> {
                    static SINGLE_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r#".*\(([0-9]*)\).*\[\]$"#).unwrap());

                    SINGLE_REGEX
                        .captures(formatted_type)
                        .and_then(|cap| cap.get(1).map(|precision| from_str::<u32>(precision.as_str()).unwrap()))
                }

                fn get_dual(formatted_type: &str) -> (Option<u32>, Option<u32>) {
                    static DUAL_REGEX: Lazy<Regex> =
                        Lazy::new(|| Regex::new(r#"numeric\(([0-9]*),([0-9]*)\)\[\]$"#).unwrap());
                    let first = DUAL_REGEX
                        .captures(formatted_type)
                        .and_then(|cap| cap.get(1).map(|precision| from_str::<u32>(precision.as_str()).unwrap()));

                    let second = DUAL_REGEX
                        .captures(formatted_type)
                        .and_then(|cap| cap.get(2).map(|precision| from_str::<u32>(precision.as_str()).unwrap()));

                    (first, second)
                }

                let formatted_type = col.get_expect_string("formatted_type");
                let fdt = col.get_expect_string("full_data_type");
                let char_max_length = match fdt.as_str() {
                    "_bpchar" | "_varchar" | "_bit" | "_varbit" => get_single(&formatted_type),
                    _ => None,
                };
                let (num_precision, num_scale) = match fdt.as_str() {
                    "_numeric" => get_dual(&formatted_type),
                    _ => (None, None),
                };
                let time = match fdt.as_str() {
                    "_timestamptz" | "_timestamp" | "_timetz" | "_time" | "_interval" => get_single(&formatted_type),
                    _ => None,
                };

                (char_max_length, num_precision, num_scale, time)
            } else {
                (
                    col.get_u32("character_maximum_length"),
                    col.get_u32("numeric_precision"),
                    col.get_u32("numeric_scale"),
                    col.get_u32("datetime_precision"),
                )
            };

        Precision {
            character_maximum_length,
            numeric_precision,
            numeric_scale,
            time_precision,
        }
    }

    /// Returns a map from table name to foreign keys.
    async fn get_foreign_keys(&self, schema: &str) -> DescriberResult<HashMap<String, Vec<ForeignKey>>> {
        // The `generate_subscripts` in the inner select is needed because the optimizer is free to reorganize the unnested rows if not explicitly ordered.
        let sql = r#"
            SELECT con.oid         as "con_id",
                att2.attname    as "child_column",
                cl.relname      as "parent_table",
                att.attname     as "parent_column",
                con.confdeltype,
                con.confupdtype,
                rel_ns.nspname as "referenced_schema_name",
                conname         as constraint_name,
                child,
                parent,
                table_name
            FROM (SELECT unnest(con1.conkey)                 as "parent",
                        unnest(con1.confkey)                as "child",
                        cl.relname                          AS table_name,
                        ns.nspname                          AS schema_name,
                        generate_subscripts(con1.conkey, 1) AS colidx,
                        con1.oid,
                        con1.confrelid,
                        con1.conrelid,
                        con1.conname,
                        con1.confdeltype,
                        con1.confupdtype
                FROM pg_class cl
                        join pg_constraint con1 on con1.conrelid = cl.oid
                        join pg_namespace ns on cl.relnamespace = ns.oid
                WHERE
                    ns.nspname = $1
                    and con1.contype = 'f'
                ORDER BY colidx
                ) con
                    JOIN pg_attribute att on att.attrelid = con.confrelid and att.attnum = con.child
                    JOIN pg_class cl on cl.oid = con.confrelid
                    JOIN pg_attribute att2 on att2.attrelid = con.conrelid and att2.attnum = con.parent
                    JOIN pg_class rel_cl on con.confrelid = rel_cl.oid
                    JOIN pg_namespace rel_ns on rel_cl.relnamespace = rel_ns.oid
            ORDER BY con_id, con.colidx;
        "#;

        // One foreign key with multiple columns will be represented here as several
        // rows with the same ID, which we will have to combine into corresponding foreign key
        // objects.
        let result_set = self.conn.query_raw(&sql, &[schema.into()]).await?;
        let mut intermediate_fks: HashMap<i64, (String, ForeignKey)> = HashMap::new();
        for row in result_set.into_iter() {
            trace!("Got description FK row {:?}", row);
            let id = row.get_expect_i64("con_id");
            let column = row.get_expect_string("child_column");
            let referenced_table = row.get_expect_string("parent_table");
            let referenced_column = row.get_expect_string("parent_column");
            let table_name = row.get_expect_string("table_name");
            let confdeltype = row.get_expect_char("confdeltype");
            let confupdtype = row.get_expect_char("confupdtype");
            let constraint_name = row.get_expect_string("constraint_name");

            let referenced_schema_name = row.get_expect_string("referenced_schema_name");

            if schema != referenced_schema_name {
                return Err(DescriberError::from(DescriberErrorKind::CrossSchemaReference {
                    from: format!("{}.{}", schema, table_name),
                    to: format!("{}.{}", referenced_schema_name, referenced_table),
                    constraint: constraint_name,
                }));
            }

            let on_delete_action = match confdeltype {
                'a' => ForeignKeyAction::NoAction,
                'r' => ForeignKeyAction::Restrict,
                'c' => ForeignKeyAction::Cascade,
                'n' => ForeignKeyAction::SetNull,
                'd' => ForeignKeyAction::SetDefault,
                _ => panic!(format!("unrecognized foreign key action '{}'", confdeltype)),
            };
            let on_update_action = match confupdtype {
                'a' => ForeignKeyAction::NoAction,
                'r' => ForeignKeyAction::Restrict,
                'c' => ForeignKeyAction::Cascade,
                'n' => ForeignKeyAction::SetNull,
                'd' => ForeignKeyAction::SetDefault,
                _ => panic!(format!("unrecognized foreign key action '{}'", confdeltype)),
            };
            match intermediate_fks.get_mut(&id) {
                Some((_, fk)) => {
                    fk.columns.push(column);
                    fk.referenced_columns.push(referenced_column);
                }
                None => {
                    let fk = ForeignKey {
                        constraint_name: Some(constraint_name),
                        columns: vec![column],
                        referenced_table,
                        referenced_columns: vec![referenced_column],
                        on_delete_action,
                        on_update_action,
                    };
                    intermediate_fks.insert(id, (table_name, fk));
                }
            };
        }

        let mut fks = HashMap::new();

        for (table_name, fk) in intermediate_fks.into_iter().map(|(_k, v)| v) {
            let entry = fks.entry(table_name).or_insert_with(Vec::new);

            trace!(
                "Found foreign key - column(s): {:?}, to table: '{}', to column(s): {:?}",
                fk.columns,
                fk.referenced_table,
                fk.referenced_columns
            );

            entry.push(fk);
        }

        for fks in fks.values_mut() {
            fks.sort_unstable_by_key(|fk| fk.columns.clone());
        }

        Ok(fks)
    }

    /// Returns a map from table name to indexes and (optional) primary key.
    async fn get_indices(
        &self,
        schema: &str,
        sequences: &[Sequence],
        columns: &HashMap<String, Vec<Column>>,
    ) -> DescriberResult<HashMap<String, (Vec<Index>, Option<PrimaryKey>)>> {
        let mut indexes_map = HashMap::new();

        let sql = r#"
        SELECT
            indexInfos.relname as name,
            columnInfos.attname AS column_name,
            rawIndex.indisunique AS is_unique,
            rawIndex.indisprimary AS is_primary_key,
            tableInfos.relname AS table_name,
            rawIndex.indkeyidx,
            pg_get_serial_sequence('"' || $1 || '"."' || tableInfos.relname || '"', columnInfos.attname) AS sequence_name
        FROM
            -- pg_class stores infos about tables, indices etc: https://www.postgresql.org/docs/current/catalog-pg-class.html
            pg_class tableInfos,
            pg_class indexInfos,
            -- pg_index stores indices: https://www.postgresql.org/docs/current/catalog-pg-index.html
            (
                SELECT
                    indrelid,
                    indexrelid,
                    indisunique,
                    indisprimary,
                    pg_index.indkey AS indkey,
                    generate_subscripts(pg_index.indkey, 1) AS indkeyidx
                FROM pg_index
                -- ignores partial indexes
                Where indpred is Null
                GROUP BY indrelid, indexrelid, indisunique, indisprimary, indkeyidx, indkey
                ORDER BY indrelid, indexrelid, indkeyidx
            ) rawIndex,
            -- pg_attribute stores infos about columns: https://www.postgresql.org/docs/current/catalog-pg-attribute.html
            pg_attribute columnInfos,
            -- pg_namespace stores info about the schema
            pg_namespace schemaInfo
        WHERE
            -- find table info for index
            tableInfos.oid = rawIndex.indrelid
            -- find index info
            AND indexInfos.oid = rawIndex.indexrelid
            -- find table columns
            AND columnInfos.attrelid = tableInfos.oid
            AND columnInfos.attnum = rawIndex.indkey[rawIndex.indkeyidx]
            -- we only consider ordinary tables
            AND tableInfos.relkind = 'r'
            -- we only consider stuff out of one specific schema
            AND tableInfos.relnamespace = schemaInfo.oid
            AND schemaInfo.nspname = $1
        GROUP BY tableInfos.relname, indexInfos.relname, rawIndex.indisunique, rawIndex.indisprimary, columnInfos.attname, rawIndex.indkeyidx
        ORDER BY rawIndex.indkeyidx
        "#;

        let rows = self.conn.query_raw(&sql, &[schema.into()]).await?;

        for row in rows {
            trace!("Got index: {:?}", row);
            let index_name = row.get_expect_string("name");
            let column_name = row.get_expect_string("column_name");
            let is_unique = row.get_expect_bool("is_unique");
            let is_primary_key = row.get_expect_bool("is_primary_key");
            let table_name = row.get_expect_string("table_name");
            let sequence_name = row.get_string("sequence_name");

            let table = columns
                .get(&table_name)
                .ok_or_else(|| format!("Index {} belongs to an unknown table ({})", index_name, table_name))
                .unwrap();
            let column_idx_in_table = table
                .iter()
                .position(|col| col.name == column_name)
                .ok_or_else(|| format!("Index {} refers to unknown column {}", index_name, column_name))
                .unwrap();

            if is_primary_key {
                let entry: &mut (Vec<_>, Option<PrimaryKey>) =
                    indexes_map.entry(table_name).or_insert_with(|| (Vec::new(), None));

                match entry.1.as_mut() {
                    Some(pk) => {
                        pk.columns.push(column_name);
                    }
                    None => {
                        let sequence = sequence_name.and_then(|sequence_name| {
                            let captures = RE_SEQ.captures(&sequence_name).expect("get captures");
                            let sequence_name = captures.get(1).expect("get capture").as_str();
                            sequences.iter().find(|s| s.name == sequence_name).map(|sequence| {
                                trace!("Got sequence corresponding to primary key: {:#?}", sequence);
                                sequence.clone()
                            })
                        });

                        entry.1 = Some(PrimaryKey {
                            columns: vec![column_name],
                            sequence,
                            constraint_name: Some(index_name.clone()),
                        });
                    }
                }
            } else {
                let entry: &mut (Vec<Index>, _) = indexes_map.entry(table_name).or_insert_with(|| (Vec::new(), None));

                if let Some(existing_index) = entry.0.iter_mut().find(|idx| idx.name == index_name) {
                    existing_index.columns.push(column_idx_in_table);
                } else {
                    entry.0.push(Index {
                        name: index_name,
                        columns: vec![column_idx_in_table],
                        tpe: match is_unique {
                            true => IndexType::Unique,
                            false => IndexType::Normal,
                        },
                    })
                }
            }
        }

        Ok(indexes_map)
    }

    #[tracing::instrument]
    async fn get_sequences(&self, schema: &str) -> DescriberResult<Vec<Sequence>> {
        let sql = "SELECT sequence_name
                  FROM information_schema.sequences
                  WHERE sequence_schema = $1";
        let rows = self.conn.query_raw(&sql, &[schema.into()]).await?;
        let sequences = rows
            .into_iter()
            .map(|seq| {
                trace!("Got sequence: {:?}", seq);
                Sequence {
                    name: seq.get_expect_string("sequence_name"),
                }
            })
            .collect();

        trace!("Found sequences: {:?}", sequences);
        Ok(sequences)
    }

    #[tracing::instrument]
    async fn get_enums(&self, schema: &str) -> DescriberResult<Vec<Enum>> {
        let sql = "
            SELECT t.typname as name, e.enumlabel as value
            FROM pg_type t
            JOIN pg_enum e ON t.oid = e.enumtypid
            JOIN pg_catalog.pg_namespace n ON n.oid = t.typnamespace
            WHERE n.nspname = $1
            ORDER BY e.enumsortorder";

        let rows = self.conn.query_raw(&sql, &[schema.into()]).await?;
        let mut enum_values: HashMap<String, Vec<String>> = HashMap::new();

        for row in rows.into_iter() {
            trace!("Got enum row: {:?}", row);
            let name = row.get_expect_string("name");
            let value = row.get_expect_string("value");

            let values = enum_values.entry(name).or_insert_with(Vec::new);
            values.push(value);
        }

        let mut enums: Vec<Enum> = enum_values
            .into_iter()
            .map(|(k, v)| Enum { name: k, values: v })
            .collect();

        enums.sort_by(|a, b| Ord::cmp(&a.name, &b.name));

        trace!("Found enums: {:?}", enums);

        Ok(enums)
    }

    fn get_default_value(
        col: &ResultRow,
        tpe: &ColumnType,
        data_type: &str,
        sequences: &[Sequence],
    ) -> Option<DefaultValue> {
        match col.get("column_default") {
            None => None,
            Some(param_value) => match param_value.to_string() {
                None => None,
                Some(x) if x.starts_with("NULL") => None,
                Some(default_string) => {
                    Some(match &tpe.family {
                        ColumnTypeFamily::Int => match Self::parse_int(&default_string) {
                            Some(int_value) => DefaultValue::value(int_value),
                            None => match is_autoincrement(&default_string, sequences) {
                                Some(seq) => DefaultValue::sequence(seq),
                                None => DefaultValue::db_generated(default_string),
                            },
                        },
                        ColumnTypeFamily::BigInt => match Self::parse_big_int(&default_string) {
                            Some(int_value) => DefaultValue::value(int_value),
                            None => match is_autoincrement(&default_string, sequences) {
                                Some(seq) => DefaultValue::sequence(seq),
                                None => DefaultValue::db_generated(default_string),
                            },
                        },
                        ColumnTypeFamily::Float => match Self::parse_float(&default_string) {
                            Some(float_value) => DefaultValue::value(float_value),
                            None => DefaultValue::db_generated(default_string),
                        },
                        ColumnTypeFamily::Decimal => match Self::parse_float(&default_string) {
                            Some(float_value) => DefaultValue::value(float_value),
                            None => DefaultValue::db_generated(default_string),
                        },
                        ColumnTypeFamily::Boolean => match Self::parse_bool(&default_string) {
                            Some(bool_value) => DefaultValue::value(bool_value),
                            None => DefaultValue::db_generated(default_string),
                        },
                        ColumnTypeFamily::String => {
                            match unsuffix_default_literal(&default_string, data_type, &tpe.full_data_type) {
                                Some(default_literal) => {
                                    DefaultValue::value(process_string_literal(default_literal.as_ref()).into_owned())
                                }
                                None => DefaultValue::db_generated(default_string),
                            }
                        }
                        ColumnTypeFamily::DateTime => {
                            match default_string.to_lowercase().as_str() {
                                "now()" | "current_timestamp" => DefaultValue::now(),
                                _ => DefaultValue::db_generated(default_string), //todo parse values
                            }
                        }
                        ColumnTypeFamily::Binary => DefaultValue::db_generated(default_string),
                        // JSON/JSONB defaults come in the '{}'::jsonb form.
                        ColumnTypeFamily::Json => unsuffix_default_literal(&default_string, "jsonb", "jsonb")
                            .or_else(|| unsuffix_default_literal(&default_string, "json", "json"))
                            .map(|default| DefaultValue::value(PrismaValue::Json(unquote_string(&default))))
                            .unwrap_or_else(move || DefaultValue::db_generated(default_string)),
                        ColumnTypeFamily::Uuid => DefaultValue::db_generated(default_string),
                        ColumnTypeFamily::Enum(enum_name) => {
                            let enum_suffix_without_quotes = format!("::{}", enum_name);
                            let enum_suffix_with_quotes = format!("::\"{}\"", enum_name);
                            if default_string.ends_with(&enum_suffix_with_quotes) {
                                DefaultValue::value(PrismaValue::Enum(Self::unquote_string(
                                    &default_string.replace(&enum_suffix_with_quotes, ""),
                                )))
                            } else if default_string.ends_with(&enum_suffix_without_quotes) {
                                DefaultValue::value(PrismaValue::Enum(Self::unquote_string(
                                    &default_string.replace(&enum_suffix_without_quotes, ""),
                                )))
                            } else {
                                DefaultValue::db_generated(default_string)
                            }
                        }
                        ColumnTypeFamily::Unsupported(_) => DefaultValue::db_generated(default_string),
                    })
                }
            },
        }
    }
}

fn get_column_type(row: &ResultRow, enums: &[Enum]) -> ColumnType {
    use ColumnTypeFamily::*;
    let data_type = row.get_expect_string("data_type");
    let full_data_type = row.get_expect_string("full_data_type");
    let is_required = match row.get_expect_string("is_nullable").to_lowercase().as_ref() {
        "no" => true,
        "yes" => false,
        x => panic!(format!("unrecognized is_nullable variant '{}'", x)),
    };

    let arity = match matches!(data_type.as_str(), "ARRAY") {
        true => ColumnArity::List,
        false if is_required => ColumnArity::Required,
        false => ColumnArity::Nullable,
    };

    let precision = SqlSchemaDescriber::get_precision(&row);
    let unsupported_type = || (Unsupported(full_data_type.clone()), None);
    let enum_exists = |name| enums.iter().any(|e| e.name == name);

    let (family, native_type) = match full_data_type.as_str() {
        name if data_type == "USER-DEFINED" && enum_exists(name) => (Enum(name.to_owned()), None),
        name if data_type == "ARRAY" && name.starts_with('_') && enum_exists(name.trim_start_matches('_')) => {
            (Enum(name.trim_start_matches('_').to_owned()), None)
        }
        "int2" | "_int2" => (Int, Some(PostgresType::SmallInt)),
        "int4" | "_int4" => (Int, Some(PostgresType::Integer)),
        "int8" | "_int8" => (BigInt, Some(PostgresType::BigInt)),
        "oid" | "_oid" => (Int, Some(PostgresType::Oid)),
        "float4" | "_float4" => (Float, Some(PostgresType::Real)),
        "float8" | "_float8" => (Float, Some(PostgresType::DoublePrecision)),
        "bool" | "_bool" => (Boolean, Some(PostgresType::Boolean)),
        "text" | "_text" => (String, Some(PostgresType::Text)),
        "citext" | "_citext" => (String, Some(PostgresType::Citext)),
        "varchar" | "_varchar" => (String, Some(PostgresType::VarChar(precision.character_maximum_length))),
        "bpchar" | "_bpchar" => (String, Some(PostgresType::Char(precision.character_maximum_length))),
        "date" | "_date" => (DateTime, Some(PostgresType::Date)),
        "bytea" | "_bytea" => (Binary, Some(PostgresType::ByteA)),
        "json" | "_json" => (Json, Some(PostgresType::JSON)),
        "jsonb" | "_jsonb" => (Json, Some(PostgresType::JSONB)),
        "uuid" | "_uuid" => (Uuid, Some(PostgresType::UUID)),
        "xml" | "_xml" => (String, Some(PostgresType::Xml)),
        // bit and varbit should be binary, but are currently mapped to strings.
        "bit" | "_bit" => (String, Some(PostgresType::Bit(precision.character_maximum_length))),
        "varbit" | "_varbit" => (String, Some(PostgresType::VarBit(precision.character_maximum_length))),
        "numeric" | "_numeric" => (
            Decimal,
            Some(PostgresType::Decimal(
                match (precision.numeric_precision, precision.numeric_scale) {
                    (None, None) => None,
                    (Some(prec), Some(scale)) => Some((prec, scale)),
                    _ => None,
                },
            )),
        ),
        "money" | "_money" => (Decimal, Some(PostgresType::Money)),
        "pg_lsn" | "_pg_lsn" => unsupported_type(),
        "time" | "_time" => (DateTime, Some(PostgresType::Time(precision.time_precision))),
        "timetz" | "_timetz" => (DateTime, Some(PostgresType::Timetz(precision.time_precision))),
        "timestamp" | "_timestamp" => (DateTime, Some(PostgresType::Timestamp(precision.time_precision))),
        "timestamptz" | "_timestamptz" => (DateTime, Some(PostgresType::Timestamptz(precision.time_precision))),
        "tsquery" | "_tsquery" => unsupported_type(),
        "tsvector" | "_tsvector" => unsupported_type(),
        "txid_snapshot" | "_txid_snapshot" => unsupported_type(),
        "inet" | "_inet" => (String, Some(PostgresType::Inet)),
        //geometric
        "box" | "_box" => unsupported_type(),
        "circle" | "_circle" => unsupported_type(),
        "line" | "_line" => unsupported_type(),
        "lseg" | "_lseg" => unsupported_type(),
        "path" | "_path" => unsupported_type(),
        "polygon" | "_polygon" => unsupported_type(),
        _ => unsupported_type(),
    };

    ColumnType {
        full_data_type,
        family,
        arity,
        native_type: native_type.map(|x| x.to_json()),
    }
}

static RE_SEQ: Lazy<Regex> = Lazy::new(|| Regex::new("^(?:.+\\.)?\"?([^.\"]+)\"?").expect("compile regex"));

static AUTOINCREMENT_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"nextval\((\(?)'((.+)\.)?(("(?P<sequence>.+)")|(?P<sequence2>.+))'(::text\))?::regclass\)"#)
        .expect("compile autoincrement regex")
});

/// Returns the name of the sequence in the schema that the defaultvalue matches if it is drawn from one of them
fn is_autoincrement(value: &str, sequences: &[Sequence]) -> Option<String> {
    AUTOINCREMENT_REGEX.captures(value).and_then(|captures| {
        let sequence_name = captures.name("sequence").or_else(|| captures.name("sequence2"));

        sequence_name.and_then(|name| {
            sequences
                .iter()
                .find(|seq| seq.name == name.as_str())
                .map(|x| x.name.clone())
        })
    })
}

fn unsuffix_default_literal<'a>(literal: &'a str, data_type: &str, full_data_type: &str) -> Option<Cow<'a, str>> {
    static POSTGRES_DATA_TYPE_SUFFIX_RE: Lazy<Regex> =
        Lazy::new(|| Regex::new(r#"(?ms)^(.*)::(\\")?(.*)(\\")?$"#).unwrap());

    let captures = POSTGRES_DATA_TYPE_SUFFIX_RE.captures(literal)?;
    let suffix = captures.get(3).unwrap().as_str();

    if suffix != data_type && suffix != full_data_type {
        return None;
    }

    let first_capture = captures.get(1).unwrap().as_str();

    Some(first_capture.into())
}

// See https://www.postgresql.org/docs/9.3/sql-syntax-lexical.html
fn process_string_literal(literal: &str) -> Cow<'_, str> {
    static POSTGRES_STRING_DEFAULT_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r#"(?ms)^B?'(.*)'$"#).unwrap());
    static POSTGRES_DEFAULT_QUOTE_UNESCAPE_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r#"'(')"#).unwrap());
    static POSTGRES_DEFAULT_BACKSLASH_UNESCAPE_RE: Lazy<Regex> =
        Lazy::new(|| Regex::new(r#"\\(["']|\\[^\\])"#).unwrap());
    static POSTGRES_STRING_DEFAULTS_PIPELINE: &[(&Lazy<Regex>, &str)] = &[
        (&POSTGRES_STRING_DEFAULT_RE, "$1"),
        (&POSTGRES_DEFAULT_QUOTE_UNESCAPE_RE, "$1"),
        (&POSTGRES_DEFAULT_BACKSLASH_UNESCAPE_RE, "$1"),
    ];

    chain_replaces(literal, POSTGRES_STRING_DEFAULTS_PIPELINE)
}

fn chain_replaces<'a>(s: &'a str, replaces: &[(&Lazy<Regex>, &str)]) -> Cow<'a, str> {
    let mut out = Cow::Borrowed(s);

    for (re, replacement) in replaces.iter() {
        if !re.is_match(out.as_ref()) {
            continue;
        }

        let replaced = re.replace_all(out.as_ref(), *replacement);

        out = Cow::Owned(replaced.into_owned())
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn postgres_is_autoincrement_works() {
        let sequences = vec![
            Sequence {
                name: "first_sequence".to_string(),
            },
            Sequence {
                name: "second_sequence".to_string(),
            },
            Sequence {
                name: "third_Sequence".to_string(),
            },
            Sequence {
                name: "fourth_Sequence".to_string(),
            },
            Sequence {
                name: "fifth_sequence".to_string(),
            },
        ];

        let first_autoincrement = r#"nextval('first_sequence'::regclass)"#;
        assert!(is_autoincrement(first_autoincrement, &sequences).is_some());

        let second_autoincrement = r#"nextval('schema_name.second_sequence'::regclass)"#;
        assert!(is_autoincrement(second_autoincrement, &sequences).is_some());

        let third_autoincrement = r#"nextval('"third_Sequence"'::regclass)"#;
        assert!(is_autoincrement(third_autoincrement, &sequences).is_some());

        let fourth_autoincrement = r#"nextval('"schema_Name"."fourth_Sequence"'::regclass)"#;
        assert!(is_autoincrement(fourth_autoincrement, &sequences).is_some());

        let fifth_autoincrement = r#"nextval(('fifth_sequence'::text)::regclass)"#;
        assert!(is_autoincrement(fifth_autoincrement, &sequences).is_some());

        let non_autoincrement = r#"string_default_named_seq"#;
        assert!(is_autoincrement(non_autoincrement, &sequences).is_none());
    }
}
