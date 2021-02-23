use super::*;
use crate::{getters::Getter, parsers::Parser};
use bigdecimal::ToPrimitive;
use indoc::indoc;
use native_types::{MySqlType, NativeType};
use quaint::{prelude::Queryable, single::Quaint, Value};
use serde_json::from_str;
use std::{
    borrow::Cow,
    collections::{BTreeMap, HashMap, HashSet},
};
use tracing::trace;

fn is_mariadb(version: &str) -> bool {
    version.contains("MariaDB")
}

enum Flavour {
    Mysql,
    MariaDb,
}

impl Flavour {
    fn from_version(version_string: &str) -> Self {
        if is_mariadb(version_string) {
            Self::MariaDb
        } else {
            Self::Mysql
        }
    }
}

pub struct SqlSchemaDescriber {
    conn: Quaint,
}

#[async_trait::async_trait]
impl super::SqlSchemaDescriberBackend for SqlSchemaDescriber {
    async fn list_databases(&self) -> DescriberResult<Vec<String>> {
        self.get_databases().await
    }

    async fn get_metadata(&self, schema: &str) -> DescriberResult<SQLMetadata> {
        let table_count = self.get_table_names(&schema).await?.len();
        let size_in_bytes = self.get_size(&schema).await?;

        Ok(SQLMetadata {
            table_count,
            size_in_bytes,
        })
    }

    #[tracing::instrument(skip(self))]
    async fn describe(&self, schema: &str) -> DescriberResult<SqlSchema> {
        let version = self.conn.version().await.ok().flatten();
        let flavour = version
            .as_ref()
            .map(|s| Flavour::from_version(s))
            .unwrap_or(Flavour::Mysql);

        let table_names = self.get_table_names(schema).await?;
        let mut tables = Vec::with_capacity(table_names.len());
        let mut columns = Self::get_all_columns(&self.conn, schema, &flavour).await?;
        let mut indexes = Self::get_all_indexes(&self.conn, schema, &columns).await?;
        let mut fks = Self::get_foreign_keys(&self.conn, schema).await?;

        let mut enums = vec![];
        for table_name in &table_names {
            let (table, enms) = self.get_table(table_name, &mut columns, &mut indexes, &mut fks);
            tables.push(table);
            enums.extend(enms.iter().cloned());
        }

        let views = self.get_views(schema).await?;
        let procedures = self.get_procedures(schema).await?;

        Ok(SqlSchema {
            tables,
            enums,
            views,
            procedures,
            sequences: vec![],
        })
    }

    #[tracing::instrument(skip(self))]
    async fn version(&self, schema: &str) -> crate::DescriberResult<Option<String>> {
        Ok(self.conn.version().await?)
    }
}

impl Parser for SqlSchemaDescriber {}

impl SqlSchemaDescriber {
    /// Constructor.
    pub fn new(conn: Quaint) -> SqlSchemaDescriber {
        SqlSchemaDescriber { conn }
    }

    #[tracing::instrument(skip(self))]
    async fn get_databases(&self) -> DescriberResult<Vec<String>> {
        let sql = "select schema_name as schema_name from information_schema.schemata;";
        let rows = self.conn.query_raw(sql, &[]).await?;
        let names = rows
            .into_iter()
            .map(|row| row.get_expect_string("schema_name"))
            .collect();

        trace!("Found schema names: {:?}", names);

        Ok(names)
    }

    #[tracing::instrument(skip(self))]
    async fn get_views(&self, schema: &str) -> DescriberResult<Vec<View>> {
        let sql = indoc! {r#"
            SELECT TABLE_NAME AS view_name, VIEW_DEFINITION AS view_sql
            FROM INFORMATION_SCHEMA.VIEWS
            WHERE TABLE_SCHEMA = ?;
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

    #[tracing::instrument(skip(self))]
    async fn get_procedures(&self, schema: &str) -> DescriberResult<Vec<Procedure>> {
        let sql = r#"
            SELECT routine_name AS name,
                routine_definition AS definition
            FROM information_schema.routines
            WHERE ROUTINE_SCHEMA = ?
            AND ROUTINE_TYPE = 'PROCEDURE'
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

    #[tracing::instrument(skip(self))]
    async fn get_table_names(&self, schema: &str) -> DescriberResult<Vec<String>> {
        let sql = "SELECT table_name as table_name FROM information_schema.tables
            WHERE table_schema = ?
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

    #[tracing::instrument(skip(self))]
    async fn get_size(&self, schema: &str) -> DescriberResult<usize> {
        let sql = r#"
            SELECT
            SUM(data_length + index_length) as size
            FROM information_schema.TABLES
            WHERE table_schema = ?
        "#;

        let result = self.conn.query_raw(sql, &[schema.into()]).await?;
        let size = result
            .first()
            .and_then(|row| {
                row.get("size")
                    .and_then(|x| x.as_numeric())
                    .and_then(|decimal| decimal.round(0).to_usize())
            })
            .unwrap_or(0);

        trace!("Found db size: {:?}", size);

        Ok(size as usize)
    }

    #[tracing::instrument(skip(self, columns, indexes, foreign_keys))]
    fn get_table(
        &self,
        name: &str,
        columns: &mut HashMap<String, (Vec<Column>, Vec<Enum>)>,
        indexes: &mut HashMap<String, (BTreeMap<String, Index>, Option<PrimaryKey>)>,
        foreign_keys: &mut HashMap<String, Vec<ForeignKey>>,
    ) -> (Table, Vec<Enum>) {
        let (columns, enums) = columns.remove(name).unwrap_or((vec![], vec![]));
        let (indices, primary_key) = indexes.remove(name).unwrap_or_else(|| (BTreeMap::new(), None));

        let foreign_keys = foreign_keys.remove(name).unwrap_or_default();

        (
            Table {
                name: name.to_string(),
                columns,
                foreign_keys,
                indices: indices.into_iter().map(|(_k, v)| v).collect(),
                primary_key,
            },
            enums,
        )
    }

    async fn get_all_columns(
        conn: &dyn Queryable,
        schema_name: &str,
        flavour: &Flavour,
    ) -> DescriberResult<HashMap<String, (Vec<Column>, Vec<Enum>)>> {
        // We alias all the columns because MySQL column names are case-insensitive in queries, but the
        // information schema column names became upper-case in MySQL 8, causing the code fetching
        // the result values by column name below to fail.
        let sql = "
            SELECT
                column_name column_name,
                data_type data_type,
                column_type full_data_type,
                character_maximum_length character_maximum_length,
                numeric_precision numeric_precision,
                numeric_scale numeric_scale,
                datetime_precision datetime_precision,
                column_default column_default,
                is_nullable is_nullable,
                extra extra,
                table_name table_name
            FROM information_schema.columns
            WHERE table_schema = ?
            ORDER BY ordinal_position
        ";

        let mut map = HashMap::new();

        let rows = conn.query_raw(sql, &[schema_name.into()]).await?;

        for col in rows {
            trace!("Got column: {:?}", col);
            let table_name = col.get_expect_string("table_name");
            let name = col.get_expect_string("column_name");
            let data_type = col.get_expect_string("data_type");
            let full_data_type = col.get_expect_string("full_data_type");

            let is_nullable = col.get_expect_string("is_nullable").to_lowercase();
            let is_required = match is_nullable.as_ref() {
                "no" => true,
                "yes" => false,
                x => panic!(format!("unrecognized is_nullable variant '{}'", x)),
            };

            let arity = if is_required {
                ColumnArity::Required
            } else {
                ColumnArity::Nullable
            };

            let character_maximum_length = col.get_u32("character_maximum_length");
            let time_precision = col.get_u32("datetime_precision");
            let numeric_precision = col.get_u32("numeric_precision");
            let numeric_scale = col.get_u32("numeric_scale");

            let precision = Precision {
                character_maximum_length,
                numeric_precision,
                numeric_scale,
                time_precision,
            };

            let default_value = col.get("column_default");

            let (tpe, enum_option) = Self::get_column_type_and_enum(
                &table_name,
                &name,
                &data_type,
                &full_data_type,
                precision,
                arity,
                default_value,
            );
            let extra = col.get_expect_string("extra").to_lowercase();
            let auto_increment = matches!(extra.as_str(), "auto_increment");

            let entry = map.entry(table_name).or_insert((Vec::new(), Vec::new()));

            if let Some(enm) = enum_option {
                entry.1.push(enm);
            }

            let default = match default_value {
                None => None,
                Some(param_value) => match param_value.to_string() {
                    None => None,
                    Some(x) if x == "NULL" => None,
                    Some(default_string) => {
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
                                _ => DefaultValue::db_generated(default_string),
                            },
                            ColumnTypeFamily::String => DefaultValue::value(PrismaValue::String(
                                Self::unescape_and_unquote_default_string(default_string, flavour),
                            )),
                            //todo check other now() definitions
                            ColumnTypeFamily::DateTime => match Self::default_is_current_timestamp(&default_string) {
                                true => DefaultValue::now(),
                                _ => DefaultValue::db_generated(default_string),
                            },
                            ColumnTypeFamily::Binary => DefaultValue::db_generated(default_string),
                            ColumnTypeFamily::Json => DefaultValue::db_generated(default_string),
                            ColumnTypeFamily::Uuid => DefaultValue::db_generated(default_string),
                            ColumnTypeFamily::Enum(_) => DefaultValue::value(PrismaValue::Enum(Self::unquote_string(
                                &default_string.replace("_utf8mb4", "").replace("\\\'", ""),
                            ))),
                            ColumnTypeFamily::Unsupported(_) => DefaultValue::db_generated(default_string),
                        })
                    }
                },
            };

            let col = Column {
                name,
                tpe,
                default,
                auto_increment,
            };

            entry.0.push(col);
        }

        Ok(map)
    }

    async fn get_all_indexes(
        conn: &dyn Queryable,
        schema_name: &str,
        columns: &HashMap<String, (Vec<Column>, Vec<Enum>)>,
    ) -> DescriberResult<HashMap<String, (BTreeMap<String, Index>, Option<PrimaryKey>)>> {
        let mut map = HashMap::new();
        let mut indexes_with_expressions: HashSet<(String, String)> = HashSet::new();
        let mut indexes_with_partially_covered_columns: HashSet<(String, String)> = HashSet::new();

        // We alias all the columns because MySQL column names are case-insensitive in queries, but the
        // information schema column names became upper-case in MySQL 8, causing the code fetching
        // the result values by column name below to fail.
        let sql = "
            SELECT DISTINCT
                index_name AS index_name,
                non_unique AS non_unique,
                column_name AS column_name,
                seq_in_index AS seq_in_index,
                table_name AS table_name,
                sub_part AS partial
            FROM INFORMATION_SCHEMA.STATISTICS
            WHERE table_schema = ?
            ORDER BY index_name, seq_in_index
            ";
        let rows = conn.query_raw(sql, &[schema_name.into()]).await?;

        for row in rows {
            trace!("Got index row: {:#?}", row);
            let table_name = row.get_expect_string("table_name");
            let index_name = row.get_expect_string("index_name");

            if row.get_u32("partial").is_some() {
                indexes_with_partially_covered_columns.insert((table_name.clone(), index_name.clone()));
            };

            let table = columns
                .get(&table_name)
                .ok_or_else(|| format!("Index {} belongs to an unknown table ({})", index_name, table_name))
                .unwrap();

            match row.get_string("column_name") {
                Some(column_name) => {
                    let seq_in_index = row.get_expect_i64("seq_in_index");
                    let pos = seq_in_index - 1;
                    let is_unique = !row.get_expect_bool("non_unique");

                    let column_idx_in_table = table
                        .0
                        .iter()
                        .position(|col| col.name == column_name)
                        .ok_or_else(|| format!("Index {} refers to unknown column {}", index_name, column_name))
                        .unwrap();

                    // Multi-column indices will return more than one row (with different column_name values).
                    // We cannot assume that one row corresponds to one index.
                    let (ref mut indexes_map, ref mut primary_key): &mut (_, Option<PrimaryKey>) = map
                        .entry(table_name)
                        .or_insert((BTreeMap::<String, Index>::new(), None));

                    let is_pk = index_name.eq_ignore_ascii_case("primary");

                    if is_pk {
                        trace!("Column '{}' is part of the primary key", column_name);
                        match primary_key {
                            Some(pk) => {
                                if pk.columns.len() < (pos + 1) as usize {
                                    pk.columns.resize((pos + 1) as usize, "".to_string());
                                }
                                pk.columns[pos as usize] = column_name;
                                trace!(
                                    "The primary key has already been created, added column to it: {:?}",
                                    pk.columns
                                );
                            }
                            None => {
                                trace!("Instantiating primary key");

                                primary_key.replace(PrimaryKey {
                                    columns: vec![column_name],
                                    sequence: None,
                                    constraint_name: None,
                                });
                            }
                        };
                    } else if indexes_map.contains_key(&index_name) {
                        if let Some(index) = indexes_map.get_mut(&index_name) {
                            index.columns.push(column_idx_in_table);
                        }
                    } else {
                        indexes_map.insert(
                            index_name.clone(),
                            Index {
                                name: index_name,
                                columns: vec![column_idx_in_table],
                                tpe: match is_unique {
                                    true => IndexType::Unique,
                                    false => IndexType::Normal,
                                },
                            },
                        );
                    }
                }
                None => {
                    indexes_with_expressions.insert((table_name, index_name));
                }
            }
        }

        for (table, (index_map, _)) in &mut map {
            for (tble, index_name) in &indexes_with_expressions {
                if tble == table {
                    index_map.remove(index_name);
                }
            }
            for (tble, index_name) in &indexes_with_partially_covered_columns {
                if tble == table {
                    index_map.remove(index_name);
                }
            }
        }

        Ok(map)
    }

    async fn get_foreign_keys(
        conn: &dyn Queryable,
        schema_name: &str,
    ) -> DescriberResult<HashMap<String, Vec<ForeignKey>>> {
        // Foreign keys covering multiple columns will return multiple rows, which we need to
        // merge.
        let mut map: HashMap<String, HashMap<String, ForeignKey>> = HashMap::new();

        // XXX: Is constraint_name unique? Need a way to uniquely associate rows with foreign keys
        // One should think it's unique since it's used to join information_schema.key_column_usage
        // and information_schema.referential_constraints tables in this query lifted from
        // Stack Overflow
        //
        // We alias all the columns because MySQL column names are case-insensitive in queries, but the
        // information schema column names became upper-case in MySQL 8, causing the code fetching
        // the result values by column name below to fail.
        let sql = "
            SELECT
                kcu.constraint_name constraint_name,
                kcu.column_name column_name,
                kcu.referenced_table_name referenced_table_name,
                kcu.referenced_column_name referenced_column_name,
                kcu.ordinal_position ordinal_position,
                kcu.table_name table_name,
                rc.delete_rule delete_rule,
                rc.update_rule update_rule
            FROM information_schema.key_column_usage AS kcu
            INNER JOIN information_schema.referential_constraints AS rc ON
            kcu.constraint_name = rc.constraint_name
            WHERE
                kcu.table_schema = ?
                AND rc.constraint_schema = ?
                AND kcu.referenced_column_name IS NOT NULL
            ORDER BY ordinal_position
        ";

        let result_set = conn.query_raw(sql, &[schema_name.into(), schema_name.into()]).await?;

        for row in result_set.into_iter() {
            trace!("Got description FK row {:#?}", row);
            let table_name = row.get_expect_string("table_name");
            let constraint_name = row.get_expect_string("constraint_name");
            let column = row.get_expect_string("column_name");
            let referenced_table = row.get_expect_string("referenced_table_name");
            let referenced_column = row.get_expect_string("referenced_column_name");
            let ord_pos = row.get_expect_i64("ordinal_position");
            let on_delete_action = match row.get_expect_string("delete_rule").to_lowercase().as_str() {
                "cascade" => ForeignKeyAction::Cascade,
                "set null" => ForeignKeyAction::SetNull,
                "set default" => ForeignKeyAction::SetDefault,
                "restrict" => ForeignKeyAction::Restrict,
                "no action" => ForeignKeyAction::NoAction,
                s => panic!(format!("Unrecognized on delete action '{}'", s)),
            };
            let on_update_action = match row.get_expect_string("update_rule").to_lowercase().as_str() {
                "cascade" => ForeignKeyAction::Cascade,
                "set null" => ForeignKeyAction::SetNull,
                "set default" => ForeignKeyAction::SetDefault,
                "restrict" => ForeignKeyAction::Restrict,
                "no action" => ForeignKeyAction::NoAction,
                s => panic!(format!("Unrecognized on update action '{}'", s)),
            };

            let intermediate_fks = map.entry(table_name).or_default();

            match intermediate_fks.get_mut(&constraint_name) {
                Some(fk) => {
                    let pos = ord_pos as usize - 1;
                    if fk.columns.len() <= pos {
                        fk.columns.resize(pos + 1, "".to_string());
                    }
                    fk.columns[pos] = column;
                    if fk.referenced_columns.len() <= pos {
                        fk.referenced_columns.resize(pos + 1, "".to_string());
                    }
                    fk.referenced_columns[pos] = referenced_column;
                }
                None => {
                    let fk = ForeignKey {
                        constraint_name: Some(constraint_name.clone()),
                        columns: vec![column],
                        referenced_table,
                        referenced_columns: vec![referenced_column],
                        on_delete_action,
                        on_update_action,
                    };
                    intermediate_fks.insert(constraint_name, fk);
                }
            };
        }

        let fks = map
            .into_iter()
            .map(|(k, v)| {
                let mut fks: Vec<ForeignKey> = v.into_iter().map(|(_k, v)| v).collect();

                fks.sort_unstable_by(|this, other| this.columns.cmp(&other.columns));

                (k, fks)
            })
            .collect();

        Ok(fks)
    }

    fn get_column_type_and_enum(
        table: &str,
        column_name: &str,
        data_type: &str,
        full_data_type: &str,
        precision: Precision,
        arity: ColumnArity,
        default: Option<&Value>,
    ) -> (ColumnType, Option<Enum>) {
        static UNSIGNEDNESS_RE: Lazy<Regex> = Lazy::new(|| Regex::new("(?i)unsigned$").unwrap());
        // println!("Name: {}", column_name);
        // println!("DT: {}", data_type);
        // println!("FDT: {}", full_data_type);
        // println!("Precision: {:?}", precision);
        // println!("Default: {:?}", default);

        let is_tinyint1 = || Self::extract_precision(full_data_type) == Some(1);
        let invalid_bool_default = || {
            default
                .and_then(|default| default.to_string())
                .filter(|default_string| default_string != "NULL")
                .and_then(|default_string| Self::parse_int(&default_string))
                .filter(|default_int| *default_int != PrismaValue::Int(0) && *default_int != PrismaValue::Int(1))
                .is_some()
        };

        let (family, native_type) = match data_type {
            "int" if UNSIGNEDNESS_RE.is_match(full_data_type) => (ColumnTypeFamily::Int, Some(MySqlType::UnsignedInt)),
            "int" => (ColumnTypeFamily::Int, Some(MySqlType::Int)),
            "smallint" if UNSIGNEDNESS_RE.is_match(full_data_type) => {
                (ColumnTypeFamily::Int, Some(MySqlType::UnsignedSmallInt))
            }
            "smallint" => (ColumnTypeFamily::Int, Some(MySqlType::SmallInt)),
            "tinyint" if is_tinyint1() && !invalid_bool_default() => {
                (ColumnTypeFamily::Boolean, Some(MySqlType::TinyInt))
            }
            "tinyint" if UNSIGNEDNESS_RE.is_match(full_data_type) => {
                (ColumnTypeFamily::Int, Some(MySqlType::UnsignedTinyInt))
            }
            "tinyint" => (ColumnTypeFamily::Int, Some(MySqlType::TinyInt)),
            "mediumint" if UNSIGNEDNESS_RE.is_match(full_data_type) => {
                (ColumnTypeFamily::Int, Some(MySqlType::UnsignedMediumInt))
            }
            "mediumint" => (ColumnTypeFamily::Int, Some(MySqlType::MediumInt)),
            "bigint" if UNSIGNEDNESS_RE.is_match(full_data_type) => {
                (ColumnTypeFamily::BigInt, Some(MySqlType::UnsignedBigInt))
            }
            "bigint" => (ColumnTypeFamily::BigInt, Some(MySqlType::BigInt)),
            "decimal" => (
                ColumnTypeFamily::Decimal,
                Some(MySqlType::Decimal(Some((
                    precision.numeric_precision.unwrap(),
                    precision.numeric_scale.unwrap(),
                )))),
            ),
            "float" => (ColumnTypeFamily::Float, Some(MySqlType::Float)),
            "double" => (ColumnTypeFamily::Float, Some(MySqlType::Double)),

            "char" => (
                ColumnTypeFamily::String,
                Some(MySqlType::Char(precision.character_maximum_length.unwrap())),
            ),
            "varchar" => (
                ColumnTypeFamily::String,
                Some(MySqlType::VarChar(precision.character_maximum_length.unwrap())),
            ),
            "text" => (ColumnTypeFamily::String, Some(MySqlType::Text)),
            "tinytext" => (ColumnTypeFamily::String, Some(MySqlType::TinyText)),
            "mediumtext" => (ColumnTypeFamily::String, Some(MySqlType::MediumText)),
            "longtext" => (ColumnTypeFamily::String, Some(MySqlType::LongText)),
            "enum" => (ColumnTypeFamily::Enum(format!("{}_{}", table, column_name)), None),
            "json" => (ColumnTypeFamily::Json, Some(MySqlType::Json)),
            "set" => (ColumnTypeFamily::String, None),
            //temporal
            "date" => (ColumnTypeFamily::DateTime, Some(MySqlType::Date)),
            "time" => (
                //Fixme this can either be a time or a duration -.-
                ColumnTypeFamily::DateTime,
                Some(MySqlType::Time(precision.time_precision)),
            ),
            "datetime" => (
                ColumnTypeFamily::DateTime,
                Some(MySqlType::DateTime(precision.time_precision)),
            ),
            "timestamp" => (
                ColumnTypeFamily::DateTime,
                Some(MySqlType::Timestamp(precision.time_precision)),
            ),
            "year" => (ColumnTypeFamily::Int, Some(MySqlType::Year)),
            "bit" if precision.numeric_precision == Some(1) => (
                ColumnTypeFamily::Boolean,
                Some(MySqlType::Bit(precision.numeric_precision.unwrap())),
            ),
            //01100010 01101001 01110100 01110011 00100110 01100010 01111001 01110100 01100101 01110011 00001010
            "bit" => (
                ColumnTypeFamily::Binary,
                Some(MySqlType::Bit(precision.numeric_precision.unwrap())),
            ),
            "binary" => (
                ColumnTypeFamily::Binary,
                Some(MySqlType::Binary(precision.character_maximum_length.unwrap())),
            ),
            "varbinary" => (
                ColumnTypeFamily::Binary,
                Some(MySqlType::VarBinary(precision.character_maximum_length.unwrap())),
            ),
            "blob" => (ColumnTypeFamily::Binary, Some(MySqlType::Blob)),
            "tinyblob" => (ColumnTypeFamily::Binary, Some(MySqlType::TinyBlob)),
            "mediumblob" => (ColumnTypeFamily::Binary, Some(MySqlType::MediumBlob)),
            "longblob" => (ColumnTypeFamily::Binary, Some(MySqlType::LongBlob)),
            //spatial
            "geometry" => (ColumnTypeFamily::Unsupported(full_data_type.into()), None),
            "point" => (ColumnTypeFamily::Unsupported(full_data_type.into()), None),
            "linestring" => (ColumnTypeFamily::Unsupported(full_data_type.into()), None),
            "polygon" => (ColumnTypeFamily::Unsupported(full_data_type.into()), None),
            "multipoint" => (ColumnTypeFamily::Unsupported(full_data_type.into()), None),
            "multilinestring" => (ColumnTypeFamily::Unsupported(full_data_type.into()), None),
            "multipolygon" => (ColumnTypeFamily::Unsupported(full_data_type.into()), None),
            "geometrycollection" => (ColumnTypeFamily::Unsupported(full_data_type.into()), None),
            _ => (ColumnTypeFamily::Unsupported(full_data_type.into()), None),
        };

        let tpe = ColumnType {
            full_data_type: full_data_type.to_owned(),
            family: family.clone(),
            arity,
            native_type: native_type.map(|x| x.to_json()),
        };

        match &family {
            ColumnTypeFamily::Enum(name) => (
                tpe,
                Some(Enum {
                    name: name.clone(),
                    values: Self::extract_enum_values(&full_data_type),
                }),
            ),
            _ => (tpe, None),
        }
    }

    fn extract_precision(input: &str) -> Option<u32> {
        static RE: Lazy<Regex> = Lazy::new(|| Regex::new(r#".*\(([1-9])\)"#).unwrap());
        RE.captures(input)
            .and_then(|cap| cap.get(1).map(|precision| from_str::<u32>(precision.as_str()).unwrap()))
    }

    fn extract_enum_values(full_data_type: &&str) -> Vec<String> {
        let len = &full_data_type.len() - 1;
        let vals = &full_data_type[5..len];
        vals.split(',').map(|v| unquote_string(v)).collect()
    }

    // See https://dev.mysql.com/doc/refman/8.0/en/string-literals.html
    //
    // In addition, MariaDB will return string literals with the quotes and extra backslashes around
    // control characters like `\n`.
    fn unescape_and_unquote_default_string(default: String, flavour: &Flavour) -> String {
        static MYSQL_ESCAPING_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r#"\\('|\\[^\\])|'(')"#).unwrap());
        static MARIADB_NEWLINE_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r#"\\n"#).unwrap());
        static MARIADB_DEFAULT_QUOTE_UNESCAPE_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r#"'(.*)'"#).unwrap());

        let maybe_unquoted: Cow<str> = if matches!(flavour, Flavour::MariaDb) {
            let unquoted = MARIADB_DEFAULT_QUOTE_UNESCAPE_RE
                .captures(&default)
                .and_then(|cap| cap.get(1).map(|x| x.as_str()))
                .unwrap_or(&default);

            MARIADB_NEWLINE_RE.replace_all(unquoted, "\n")
        } else {
            default.into()
        };

        MYSQL_ESCAPING_RE.replace_all(&maybe_unquoted, "$1$2").into()
    }

    /// Tests whether an introspected default value should be categorized as current_timestamp.
    fn default_is_current_timestamp(default_str: &str) -> bool {
        static MYSQL_CURRENT_TIMESTAMP_RE: Lazy<Regex> =
            Lazy::new(|| Regex::new(r#"(?i)current_timestamp(\([0-9]*\))?"#).unwrap());

        MYSQL_CURRENT_TIMESTAMP_RE.is_match(default_str)
    }
}
