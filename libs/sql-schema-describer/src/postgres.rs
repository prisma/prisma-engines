//! Postgres description.
use super::*;
use crate::getters::Getter;
use native_types::{NativeType, PostgresType};
use quaint::connector::ResultRow;
use quaint::{prelude::Queryable, single::Quaint};
use regex::Regex;
use std::{borrow::Cow, collections::HashMap, convert::TryInto};
use tracing::debug;

pub struct SqlSchemaDescriber {
    conn: Quaint,
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
        let sequences = self.get_sequences(schema).await?;
        let enums = self.get_enums(schema).await?;
        let mut columns = self.get_columns(schema, &enums).await;
        let mut foreign_keys = self.get_foreign_keys(schema).await;
        let mut indexes = self.get_indices(schema, &sequences).await;

        let table_names = self.get_table_names(schema).await;
        let mut tables = Vec::with_capacity(table_names.len());

        for table_name in &table_names {
            tables.push(self.get_table(&table_name, &mut columns, &mut foreign_keys, &mut indexes));
        }

        Ok(SqlSchema {
            enums,
            sequences,
            tables,
        })
    }

    async fn version(&self, schema: &str) -> crate::SqlSchemaDescriberResult<Option<String>> {
        debug!("getting db version '{}'", schema);
        Ok(self.conn.version().await.unwrap())
    }
}

impl SqlSchemaDescriber {
    /// Constructor.
    pub fn new(conn: Quaint) -> SqlSchemaDescriber {
        SqlSchemaDescriber { conn }
    }

    async fn get_databases(&self) -> Vec<String> {
        debug!("Getting databases");
        let sql = "select schema_name from information_schema.schemata;";
        let rows = self.conn.query_raw(sql, &[]).await.expect("get schema names ");
        let names = rows
            .into_iter()
            .map(|row| row.get_expect_string("schema_name"))
            .collect();

        debug!("Found schema names: {:?}", names);
        names
    }

    async fn get_table_names(&self, schema: &str) -> Vec<String> {
        debug!("Getting table names");
        let sql = "SELECT table_name as table_name FROM information_schema.tables
            WHERE table_schema = $1
            -- Views are not supported yet
            AND table_type = 'BASE TABLE'
            ORDER BY table_name";
        let rows = self
            .conn
            .query_raw(sql, &[schema.into()])
            .await
            .expect("get table names ");
        let names = rows
            .into_iter()
            .map(|row| row.get_expect_string("table_name"))
            .collect();

        debug!("Found table names: {:?}", names);
        names
    }

    async fn get_size(&self, schema: &str) -> usize {
        debug!("Getting db size");
        let sql =
            "SELECT SUM(pg_total_relation_size(quote_ident(schemaname) || '.' || quote_ident(tablename)))::BIGINT as size
             FROM pg_tables
             WHERE schemaname = $1::text";
        let mut result_iter = self
            .conn
            .query_raw(sql, &[schema.into()])
            .await
            .expect("get db size ")
            .into_iter();
        let size: i64 = result_iter.next().and_then(|row| row.get_i64("size")).unwrap_or(0);

        debug!("Found db size: {:?}", size);
        size.try_into().unwrap()
    }

    fn get_table(
        &self,
        name: &str,
        columns: &mut HashMap<String, Vec<Column>>,
        foreign_keys: &mut HashMap<String, Vec<ForeignKey>>,
        indices: &mut HashMap<String, (Vec<Index>, Option<PrimaryKey>)>,
    ) -> Table {
        debug!("Getting table '{}'", name);
        let (indices, primary_key) = indices.remove(name).unwrap_or_else(|| (Vec::new(), None));
        let foreign_keys = foreign_keys.remove(name).unwrap_or_else(Vec::new);
        let columns = columns.remove(name).expect("could not get columns");
        Table {
            name: name.to_string(),
            columns,
            foreign_keys,
            indices,
            primary_key,
        }
    }

    async fn get_columns(&self, schema: &str, enums: &[Enum]) -> HashMap<String, Vec<Column>> {
        let mut columns: HashMap<String, Vec<Column>> = HashMap::new();

        let sql = r#"
            SELECT
                table_name,
                column_name,
                data_type,
                udt_name as full_data_type,
                column_default,
                is_nullable,
                is_identity,
                character_maximum_length,
                numeric_precision,
                numeric_scale,
                numeric_precision_radix,
                datetime_precision
            FROM information_schema.columns
            WHERE table_schema = $1
            ORDER BY ordinal_position
        "#;

        let rows = self
            .conn
            .query_raw(&sql, &[schema.into()])
            .await
            .expect("querying for columns");

        for col in rows {
            debug!("Got column: {:?}", col);
            let table_name = col.get_expect_string("table_name");
            let col_name = col.get_expect_string("column_name");
            let data_type = col.get_expect_string("data_type");
            let full_data_type = col.get_expect_string("full_data_type");
            let is_identity_str = col.get_expect_string("is_identity").to_lowercase();
            let is_nullable = col.get_expect_string("is_nullable").to_lowercase();

            let is_identity = match is_identity_str.as_str() {
                "no" => false,
                "yes" => true,
                _ => panic!("unrecognized is_identity variant '{}'", is_identity_str),
            };
            let is_required = match is_nullable.as_ref() {
                "no" => true,
                "yes" => false,
                x => panic!(format!("unrecognized is_nullable variant '{}'", x)),
            };

            let arity = if data_type == "ARRAY" {
                ColumnArity::List
            } else if is_required {
                ColumnArity::Required
            } else {
                ColumnArity::Nullable
            };

            let precision = SqlSchemaDescriber::get_precision(&col);

            let tpe = get_column_type(data_type.as_ref(), &full_data_type, arity, enums, precision);

            let default = get_default_value(schema, col, &table_name, &col_name, &tpe);

            let auto_increment = is_identity || matches!(default, Some(DefaultValue::SEQUENCE(_)));

            let col = Column {
                name: col_name,
                tpe,
                default,
                auto_increment,
            };

            columns.entry(table_name).or_default().push(col);
        }

        debug!("Found table columns: {:?}", columns);

        columns
    }

    fn get_precision(col: &ResultRow) -> Precision {
        let character_maximum_length = col.get_u32("character_maximum_length");
        let numeric_precision = col.get_u32("numeric_precision");
        let numeric_precision_radix = col.get_u32("numeric_precision_radix");
        let numeric_scale = col.get_u32("numeric_scale");
        let time_precision = col.get_u32("datetime_precision");

        Precision {
            character_maximum_length,
            numeric_precision,
            numeric_precision_radix,
            numeric_scale,
            time_precision,
        }
    }

    /// Returns a map from table name to foreign keys.
    async fn get_foreign_keys(&self, schema: &str) -> HashMap<String, Vec<ForeignKey>> {
        // The `generate_subscripts` in the inner select is needed because the optimizer is free to reorganize the unnested rows if not explicitly ordered.
        let sql = r#"
            SELECT
                con.oid as "con_id",
                att2.attname as "child_column",
                cl.relname as "parent_table",
                att.attname as "parent_column",
                con.confdeltype,
                con.confupdtype,
                conname as constraint_name,
                child,
                parent,
                table_name
            FROM
            (SELECT
                    unnest(con1.conkey) as "parent",
                    unnest(con1.confkey) as "child",
                    cl.relname AS table_name,
                    generate_subscripts(con1.conkey, 1) AS colidx,
                    con1.oid,
                    con1.confrelid,
                    con1.conrelid,
                    con1.conname,
                    con1.confdeltype,
                    con1.confupdtype
                FROM
                    pg_class cl
                    join pg_namespace ns on cl.relnamespace = ns.oid
                    join pg_constraint con1 on con1.conrelid = cl.oid
                WHERE
                    ns.nspname = $1
                    and con1.contype = 'f'
                    ORDER BY colidx
            ) con
            JOIN pg_attribute att on
                att.attrelid = con.confrelid and att.attnum = con.child
            JOIN pg_class cl on
                cl.oid = con.confrelid
            JOIN pg_attribute att2 on
                att2.attrelid = con.conrelid and att2.attnum = con.parent
            ORDER BY con_id, con.colidx"#;

        // One foreign key with multiple columns will be represented here as several
        // rows with the same ID, which we will have to combine into corresponding foreign key
        // objects.
        let result_set = self
            .conn
            .query_raw(&sql, &[schema.into()])
            .await
            .expect("querying for foreign keys");
        let mut intermediate_fks: HashMap<i64, (String, ForeignKey)> = HashMap::new();
        for row in result_set.into_iter() {
            debug!("Got description FK row {:?}", row);
            let id = row.get_expect_i64("con_id");
            let column = row.get_expect_string("child_column");
            let referenced_table = row.get_expect_string("parent_table");
            let referenced_column = row.get_expect_string("parent_column");
            let table_name = row.get_expect_string("table_name");
            let confdeltype = row.get_expect_char("confdeltype");
            let confupdtype = row.get_expect_char("confupdtype");
            let constraint_name = row.get_expect_string("constraint_name");

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

            debug!(
                "Found foreign key - column(s): {:?}, to table: '{}', to column(s): {:?}",
                fk.columns, fk.referenced_table, fk.referenced_columns
            );

            entry.push(fk);
        }

        for fks in fks.values_mut() {
            fks.sort_unstable_by_key(|fk| fk.columns.clone());
        }

        fks
    }

    /// Returns a map from table name to indexes and (optional) primary key.
    async fn get_indices(
        &self,
        schema: &str,
        sequences: &[Sequence],
    ) -> HashMap<String, (Vec<Index>, Option<PrimaryKey>)> {
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
        let rows = self
            .conn
            .query_raw(&sql, &[schema.into()])
            .await
            .expect("querying for indices");

        for index in rows {
            debug!("Got index: {:?}", index);
            let IndexRow {
                column_name,
                is_primary_key,
                is_unique,
                name,
                sequence_name,
                table_name,
            } = quaint::serde::from_row::<IndexRow>(index).unwrap();

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
                                debug!("Got sequence corresponding to primary key: {:#?}", sequence);
                                sequence.clone()
                            })
                        });

                        entry.1 = Some(PrimaryKey {
                            columns: vec![column_name],
                            sequence,
                            constraint_name: Some(name.clone()),
                        });
                    }
                }
            } else {
                let entry: &mut (Vec<Index>, _) = indexes_map.entry(table_name).or_insert_with(|| (Vec::new(), None));

                if let Some(existing_index) = entry.0.iter_mut().find(|idx| idx.name == name) {
                    existing_index.columns.push(column_name);
                } else {
                    entry.0.push(Index {
                        name,
                        columns: vec![column_name],
                        tpe: match is_unique {
                            true => IndexType::Unique,
                            false => IndexType::Normal,
                        },
                    })
                }
            }
        }

        indexes_map
    }

    async fn get_sequences(&self, schema: &str) -> SqlSchemaDescriberResult<Vec<Sequence>> {
        debug!("Getting sequences");
        let sql = "SELECT start_value, sequence_name
                  FROM information_schema.sequences
                  WHERE sequence_schema = $1";
        let rows = self
            .conn
            .query_raw(&sql, &[schema.into()])
            .await
            .expect("querying for sequences");
        let sequences = rows
            .into_iter()
            .map(|seq| {
                debug!("Got sequence: {:?}", seq);
                let initial_value = seq
                    .get("start_value")
                    .and_then(|x| x.to_string())
                    .and_then(|x| x.parse::<u32>().ok())
                    .expect("get start_value");
                Sequence {
                    // Not sure what allocation size refers to, but the TypeScript implementation
                    // hardcodes this as 1
                    allocation_size: 1,
                    initial_value,
                    name: seq.get_expect_string("sequence_name"),
                }
            })
            .collect();

        debug!("Found sequences: {:?}", sequences);
        Ok(sequences)
    }

    async fn get_enums(&self, schema: &str) -> SqlSchemaDescriberResult<Vec<Enum>> {
        debug!("Getting enums");

        let sql = "SELECT t.typname as name, e.enumlabel as value
            FROM pg_type t
            JOIN pg_enum e ON t.oid = e.enumtypid
            JOIN pg_catalog.pg_namespace n ON n.oid = t.typnamespace
            WHERE n.nspname = $1
            ORDER BY e.enumsortorder";

        let rows = self.conn.query_raw(&sql, &[schema.into()]).await.unwrap();
        let mut enum_values: HashMap<String, Vec<String>> = HashMap::new();

        for row in rows.into_iter() {
            debug!("Got enum row: {:?}", row);
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

        debug!("Found enums: {:?}", enums);
        Ok(enums)
    }
}

#[derive(Deserialize)]
struct IndexRow {
    name: String,
    column_name: String,
    is_unique: bool,
    is_primary_key: bool,
    table_name: String,
    sequence_name: Option<String>,
}

fn get_default_value(
    schema: &str,
    col: ResultRow,
    table_name: &String,
    col_name: &String,
    tpe: &ColumnType,
) -> Option<DefaultValue> {
    match col.get("column_default") {
        None => None,
        Some(param_value) => match param_value.to_string() {
            None => None,
            Some(x) if x.starts_with("NULL") => None,
            Some(default_string) => {
                Some(match &tpe.family {
                    ColumnTypeFamily::Int => match parse_int(&default_string) {
                        Some(int_value) => DefaultValue::VALUE(int_value),
                        None => match is_autoincrement(&default_string, schema, &table_name, &col_name) {
                            true => DefaultValue::SEQUENCE(default_string),
                            false => DefaultValue::DBGENERATED(default_string),
                        },
                    },
                    ColumnTypeFamily::Float => match parse_float(&default_string) {
                        Some(float_value) => DefaultValue::VALUE(float_value),
                        None => DefaultValue::DBGENERATED(default_string),
                    },
                    ColumnTypeFamily::Decimal => match parse_float(&default_string) {
                        Some(float_value) => DefaultValue::VALUE(float_value),
                        None => DefaultValue::DBGENERATED(default_string),
                    },
                    ColumnTypeFamily::Boolean => match parse_bool(&default_string) {
                        Some(bool_value) => DefaultValue::VALUE(bool_value),
                        None => DefaultValue::DBGENERATED(default_string),
                    },
                    ColumnTypeFamily::String => {
                        match unsuffix_default_literal(&default_string, &tpe.data_type, &tpe.full_data_type) {
                            Some(default_literal) => DefaultValue::VALUE(PrismaValue::String(
                                process_string_literal(default_literal.as_ref()).into(),
                            )),
                            None => DefaultValue::DBGENERATED(default_string),
                        }
                    }
                    ColumnTypeFamily::DateTime => {
                        match default_string.to_lowercase().as_str() {
                            "now()" | "current_timestamp" => DefaultValue::NOW,
                            _ => DefaultValue::DBGENERATED(default_string), //todo parse values
                        }
                    }
                    ColumnTypeFamily::Binary => DefaultValue::DBGENERATED(default_string),
                    // JSON/JSONB defaults come in the '{}'::jsonb form.
                    ColumnTypeFamily::Json => unsuffix_default_literal(&default_string, "jsonb", "jsonb")
                        .or_else(|| unsuffix_default_literal(&default_string, "json", "json"))
                        .map(|default| DefaultValue::VALUE(PrismaValue::Json(unquote_string(&default))))
                        .unwrap_or_else(move || DefaultValue::DBGENERATED(default_string)),
                    ColumnTypeFamily::Uuid => DefaultValue::DBGENERATED(default_string),
                    ColumnTypeFamily::Xml => DefaultValue::DBGENERATED(default_string),
                    ColumnTypeFamily::Enum(enum_name) => {
                        let enum_suffix_without_quotes = format!("::{}", enum_name);
                        let enum_suffix_with_quotes = format!("::\"{}\"", enum_name);
                        if default_string.ends_with(&enum_suffix_with_quotes) {
                            DefaultValue::VALUE(PrismaValue::Enum(unquote_string(
                                &default_string.replace(&enum_suffix_with_quotes, ""),
                            )))
                        } else if default_string.ends_with(&enum_suffix_without_quotes) {
                            DefaultValue::VALUE(PrismaValue::Enum(unquote_string(
                                &default_string.replace(&enum_suffix_without_quotes, ""),
                            )))
                        } else {
                            DefaultValue::DBGENERATED(default_string)
                        }
                    }
                    ColumnTypeFamily::Duration => DefaultValue::DBGENERATED(default_string),
                    ColumnTypeFamily::Unsupported(_) => DefaultValue::DBGENERATED(default_string),
                })
            }
        },
    }
}

fn get_column_type<'a>(
    data_type: &str,
    full_data_type: &'a str,
    arity: ColumnArity,
    enums: &[Enum],
    precision: Precision,
) -> ColumnType {
    use ColumnTypeFamily::*;
    let trim = |name: &'a str| name.trim_start_matches('_');
    let enum_exists = |name: &'a str| enums.iter().any(|e| e.name == name);

    let (family, native_type) = match full_data_type {
        x if data_type == "USER-DEFINED" && enum_exists(x) => (Enum(x.to_owned()), None),
        x if data_type == "ARRAY" && x.starts_with('_') && enum_exists(trim(x)) => (Enum(trim(x).to_owned()), None),
        "int2" | "_int2" => (Int, Some(PostgresType::SmallInt)),
        "int4" | "_int4" => (Int, Some(PostgresType::Integer)),
        "int8" | "_int8" => (Int, Some(PostgresType::BigInt)),
        "oid" | "_oid" => (Int, None),
        "float4" | "_float4" => (Float, Some(PostgresType::Real)),
        "float8" | "_float8" => (Float, Some(PostgresType::DoublePrecision)),
        "bool" | "_bool" => (Boolean, Some(PostgresType::Boolean)),
        "text" | "_text" => (String, Some(PostgresType::Text)),
        "citext" | "_citext" => (String, None),
        "varchar" | "_varchar" => (String, Some(PostgresType::VarChar(precision.character_max_length()))),
        "bpchar" | "_bpchar" => (String, Some(PostgresType::Char(precision.character_max_length()))),
        "date" | "_date" => (DateTime, Some(PostgresType::Date)),
        "bytea" | "_bytea" => (Binary, Some(PostgresType::ByteA)),
        "json" | "_json" => (Json, Some(PostgresType::JSON)),
        "jsonb" | "_jsonb" => (Json, Some(PostgresType::JSONB)),
        "uuid" | "_uuid" => (Uuid, Some(PostgresType::UUID)),
        "xml" | "_xml" => (Xml, Some(PostgresType::Xml)),
        // bit and varbit should be binary, but are currently mapped to strings.
        "bit" | "_bit" => (String, Some(PostgresType::Bit(precision.character_max_length()))),
        "varbit" | "_varbit" => (String, Some(PostgresType::VarBit(precision.character_max_length()))),
        "numeric" | "_numeric" => (
            Decimal,
            Some(PostgresType::Numeric(
                precision.numeric_precision(),
                precision.numeric_scale(),
            )),
        ),
        "money" | "_money" => (Float, None),
        "pg_lsn" | "_pg_lsn" => (Unsupported(full_data_type.into()), None),
        "time" | "_time" => (DateTime, Some(PostgresType::Time(precision.time_precision()))),
        "timetz" | "_timetz" => (
            DateTime,
            Some(PostgresType::TimeWithTimeZone(precision.time_precision())),
        ),
        "interval" | "_interval" => (Duration, Some(PostgresType::Interval(precision.time_precision()))),
        "timestamp" | "_timestamp" => (DateTime, Some(PostgresType::Timestamp(precision.time_precision()))),
        "timestamptz" | "_timestamptz" => (
            DateTime,
            Some(PostgresType::TimestampWithTimeZone(precision.time_precision())),
        ),
        "tsquery" | "_tsquery" => (Unsupported(full_data_type.into()), None),
        "tsvector" | "_tsvector" => (Unsupported(full_data_type.into()), None),
        "txid_snapshot" | "_txid_snapshot" => (Unsupported(full_data_type.into()), None),
        "inet" | "_inet" => (String, None),
        //geometric
        "box" | "_box" => (Unsupported(full_data_type.into()), None),
        "circle" | "_circle" => (Unsupported(full_data_type.into()), None),
        "line" | "_line" => (Unsupported(full_data_type.into()), None),
        "lseg" | "_lseg" => (Unsupported(full_data_type.into()), None),
        "path" | "_path" => (Unsupported(full_data_type.into()), None),
        "polygon" | "_polygon" => (Unsupported(full_data_type.into()), None),
        _ => (Unsupported(full_data_type.into()), None),
    };

    ColumnType {
        data_type: data_type.to_owned(),
        full_data_type: full_data_type.to_owned(),
        character_maximum_length: precision.character_maximum_length,
        family,
        arity,
        native_type: native_type.map(|x| x.to_json()),
    }
}

static RE_SEQ: Lazy<Regex> = Lazy::new(|| Regex::new("^(?:.+\\.)?\"?([^.\"]+)\"?").expect("compile regex"));

static AUTOINCREMENT_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r#"nextval\('(?:"(?P<schema_name>.+)"\.)?(")?(?P<table_and_column_name>.+)_seq(?:[0-9]+)?(")?'::regclass\)"#,
    )
    .unwrap()
});

/// Returns whether a particular sequence (`value`) matches the provided column info.
/// todo this only seems to work on sequence names autogenerated by barrel???
/// the names for manually created and named sequences wont match
fn is_autoincrement(value: &str, schema_name: &str, table_name: &str, column_name: &str) -> bool {
    AUTOINCREMENT_REGEX
        .captures(value)
        .and_then(|captures| {
            captures
                .name("schema_name")
                .map(|matched| matched.as_str())
                .or(Some(schema_name))
                .filter(|matched| *matched == schema_name)
                .and_then(|_| {
                    captures.name("table_and_column_name").filter(|matched| {
                        let expected_len = table_name.len() + column_name.len() + 1;

                        if matched.as_str().len() != expected_len {
                            return false;
                        }

                        let table_name_segments = table_name.split('_');
                        let column_name_segments = column_name.split('_');
                        let matched_segments = matched.as_str().split('_');
                        matched_segments
                            .zip(table_name_segments.chain(column_name_segments))
                            // postgres automatically lower-cases table/column names when generating sequence names
                            .all(|(found, expected)| found == expected || found == expected.to_lowercase())
                    })
                })
                .map(|_| true)
        })
        .unwrap_or(false)
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
        let schema_name = "prisma";
        let table_name = "Test";
        let col_name = "id";

        let non_autoincrement = "_seq";
        assert!(!is_autoincrement(non_autoincrement, schema_name, table_name, col_name));

        let autoincrement = format!(
            r#"nextval('"{}"."{}_{}_seq"'::regclass)"#,
            schema_name, table_name, col_name
        );
        assert!(is_autoincrement(&autoincrement, schema_name, table_name, col_name));

        let autoincrement_with_number = format!(
            r#"nextval('"{}"."{}_{}_seq1"'::regclass)"#,
            schema_name, table_name, col_name
        );
        assert!(is_autoincrement(
            &autoincrement_with_number,
            schema_name,
            table_name,
            col_name
        ));

        let autoincrement_without_schema = format!(r#"nextval('"{}_{}_seq1"'::regclass)"#, table_name, col_name);
        assert!(is_autoincrement(
            &autoincrement_without_schema,
            schema_name,
            table_name,
            col_name
        ));

        // The table and column names contain underscores, so it's impossible to say from the sequence where one starts and the other ends.
        let autoincrement_with_ambiguous_table_and_column_names =
            r#"nextval('"compound_table_compound_column_name_seq"'::regclass)"#;
        assert!(is_autoincrement(
            &autoincrement_with_ambiguous_table_and_column_names,
            "<ignored>",
            "compound_table",
            "compound_column_name",
        ));

        // The table and column names contain underscores, so it's impossible to say from the sequence where one starts and the other ends.
        // But this one has extra text between table and column names, so it should not match.
        let autoincrement_with_ambiguous_table_and_column_names =
            r#"nextval('"compound_table_something_compound_column_name_seq"'::regclass)"#;
        assert!(!is_autoincrement(
            &autoincrement_with_ambiguous_table_and_column_names,
            "<ignored>",
            "compound_table",
            "compound_column_name",
        ));
    }
}
