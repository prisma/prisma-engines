//! MySQL schema description.

use crate::{getters::Getter, parsers::Parser, *};
use bigdecimal::ToPrimitive;
use indexmap::IndexMap;
use indoc::indoc;
use psl::{builtin_connectors::MySqlType, datamodel_connector::NativeTypeInstance};
use quaint::{
    prelude::{Queryable, ResultRow},
    Value,
};
use std::borrow::Cow;
use tracing::trace;

/// Matches a default value in the schema, wrapped single quotes.
///
/// Example:
///
/// ```ignore
/// 'this is a test'
/// ```
static DEFAULT_QUOTES: Lazy<Regex> = Lazy::new(|| Regex::new(r"'(.*)'").unwrap());

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

#[enumflags2::bitflags]
#[derive(Clone, Copy, Debug)]
#[repr(u8)]
pub enum Circumstances {
    MariaDb,
    MySql56,
    MySql57,
    CheckConstraints,
}

pub struct SqlSchemaDescriber<'a> {
    conn: &'a dyn Queryable,
    circumstances: BitFlags<Circumstances>,
}

#[async_trait::async_trait]
impl super::SqlSchemaDescriberBackend for SqlSchemaDescriber<'_> {
    async fn list_databases(&self) -> DescriberResult<Vec<String>> {
        self.get_databases().await
    }

    async fn get_metadata(&self, schema: &str) -> DescriberResult<SqlMetadata> {
        let mut sql_schema = SqlSchema::default();
        let table_count = self.get_table_names(schema, &mut sql_schema).await?.len();
        let size_in_bytes = self.get_size(schema).await?;

        Ok(SqlMetadata {
            table_count,
            size_in_bytes,
        })
    }

    #[tracing::instrument(skip(self))]
    async fn describe(&self, schemas: &[&str]) -> DescriberResult<SqlSchema> {
        let schema = schemas[0];
        let mut sql_schema = SqlSchema::default();
        let version = self.conn.version().await.ok().flatten();
        let flavour = version
            .as_ref()
            .map(|s| Flavour::from_version(s))
            .unwrap_or(Flavour::Mysql);

        sql_schema.views = self.get_views(schema).await?;

        let table_names = self.get_table_names(schema, &mut sql_schema).await?;
        sql_schema.tables.reserve(table_names.len());
        sql_schema.table_columns.reserve(table_names.len());

        self.get_constraints(&table_names, &mut sql_schema).await?;

        self.get_all_columns(&table_names, schema, &mut sql_schema, &flavour)
            .await?;
        push_foreign_keys(schema, &table_names, &mut sql_schema, self.conn).await?;
        push_indexes(&table_names, schema, &mut sql_schema, self.conn).await?;

        sql_schema.procedures = self.get_procedures(schema).await?;

        Ok(sql_schema)
    }

    #[tracing::instrument(skip(self))]
    async fn version(&self) -> crate::DescriberResult<Option<String>> {
        Ok(self.conn.version().await?)
    }
}

async fn push_indexes(
    table_ids: &IndexMap<String, TableId>,
    schema_name: &str,
    sql_schema: &mut SqlSchema,
    conn: &dyn Queryable,
) -> DescriberResult<()> {
    // We alias all the columns because MySQL column names are case-insensitive in queries, but
    // the information schema column names became upper-case in MySQL 8, causing the code
    // fetching the result values by column name below to fail.
    let sql = include_str!("mysql/indexes_query.sql");

    let rows = conn.query_raw(sql, &[schema_name.into()]).await?;
    let mut current_index_id: Option<IndexId> = None;
    let mut index_should_be_filtered_out = false;

    let remove_last_index = |schema: &mut SqlSchema, index_id: IndexId| {
        schema.indexes.pop().unwrap();
        loop {
            match schema.index_columns.last() {
                Some(col) if col.index_id == index_id => {
                    schema.index_columns.pop().unwrap();
                }
                None | Some(_) => break,
            }
        }
    };

    for row in rows {
        let table_name = row.get_expect_string("table_name");

        let table_id = if let Some(id) = table_ids.get(table_name.as_str()) {
            *id
        } else {
            continue;
        };

        let index_name = row.get_expect_string("index_name");
        let length = row.get_u32("partial");

        let sort_order = row.get_string("column_order").map(|v| match v.as_ref() {
            "A" => SQLSortOrder::Asc,
            "D" => SQLSortOrder::Desc,
            misc => panic!("Unexpected sort order `{misc}`, collation should be A, D or Null"),
        });

        let seq_in_index = row.get_expect_i64("seq_in_index"); // starts at 1

        let column_name = if let Some(name) = row.get_string("column_name") {
            name
        } else {
            // filter out indexes on expressions
            // if the sequence is 1 and we have an expression,
            // we never create an index to the collection and can
            // just continue
            index_should_be_filtered_out = seq_in_index > 1;
            continue;
        };

        let column_id = if let Some(col) = sql_schema.walk(table_id).column(&column_name) {
            col.id
        } else {
            continue;
        };

        let is_unique = !row.get_expect_bool("non_unique");
        let is_pk = index_name.eq_ignore_ascii_case("primary");
        let is_fulltext = row.get_string("index_type").as_deref() == Some("FULLTEXT");

        if seq_in_index == 1 {
            // new index!

            // first delete the old one if necessary
            if index_should_be_filtered_out {
                remove_last_index(sql_schema, current_index_id.unwrap());
                index_should_be_filtered_out = false;
            }

            // then install the new one
            let index_id = if is_pk {
                sql_schema.push_primary_key(table_id, String::new())
            } else if is_unique {
                sql_schema.push_unique_constraint(table_id, index_name)
            } else if is_fulltext {
                sql_schema.push_fulltext_index(table_id, index_name)
            } else {
                sql_schema.push_index(table_id, index_name)
            };

            current_index_id = Some(index_id);
        }

        sql_schema.push_index_column(IndexColumn {
            index_id: current_index_id.unwrap(),
            column_id,
            sort_order,
            length,
        });
    }

    if index_should_be_filtered_out {
        remove_last_index(sql_schema, current_index_id.unwrap())
    }

    Ok(())
}

impl Parser for SqlSchemaDescriber<'_> {}

impl<'a> SqlSchemaDescriber<'a> {
    /// Constructor.
    pub fn new(conn: &'a dyn Queryable, circumstances: BitFlags<Circumstances>) -> SqlSchemaDescriber<'a> {
        SqlSchemaDescriber { conn, circumstances }
    }

    #[tracing::instrument(skip(self))]
    async fn get_databases(&self) -> DescriberResult<Vec<String>> {
        let sql = "select schema_name as schema_name from information_schema.schemata;";
        let rows = self.conn.query_raw(sql, &[]).await?;
        let names = rows
            .into_iter()
            .map(|row| row.get_expect_string("schema_name"))
            .collect();

        trace!("Found schema names: {names:?}");

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
                namespace_id: NamespaceId(0),
                name: row.get_expect_string("view_name"),
                definition: row.get_string("view_sql"),
                description: None,
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
                namespace_id: NamespaceId(0),
                name: row.get_expect_string("name"),
                definition: row.get_string("definition"),
            });
        }

        Ok(procedures)
    }

    #[tracing::instrument(skip(self))]
    async fn get_table_names(
        &self,
        schema: &str,
        sql_schema: &mut SqlSchema,
    ) -> DescriberResult<IndexMap<String, TableId>> {
        // Only consider tables for which we can read at least one column.
        let sql = r#"
            SELECT DISTINCT
              BINARY table_info.table_name AS table_name,
              table_info.create_options AS create_options,
              table_info.table_comment AS table_comment
            FROM information_schema.tables AS table_info
            JOIN information_schema.columns AS column_info
                ON BINARY column_info.table_name = BINARY table_info.table_name
            WHERE
                table_info.table_schema = ?
                AND column_info.table_schema = ?
                -- Exclude views.
                AND table_info.table_type = 'BASE TABLE'
            ORDER BY BINARY table_info.table_name"#;
        let rows = self.conn.query_raw(sql, &[schema.into(), schema.into()]).await?;
        let names = rows.into_iter().map(|row| {
            (
                row.get_expect_string("table_name"),
                row.get_string("create_options")
                    .filter(|c| c.as_str() == "partitioned")
                    .is_some(),
                row.get_string("table_comment").filter(|c| !c.is_empty()),
            )
        });

        let mut map = IndexMap::default();

        for (name, is_partition, description) in names {
            let cloned_name = name.clone();
            let id = if is_partition {
                sql_schema.push_table_with_properties(
                    name,
                    Default::default(),
                    Into::into(TableProperties::IsPartition),
                    description,
                )
            } else {
                sql_schema.push_table(name, Default::default(), description)
            };
            map.insert(cloned_name, id);
        }

        trace!("Found table names: {map:?}");

        Ok(map)
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

        trace!("Found db size: {size:?}");

        Ok(size)
    }

    async fn get_all_columns(
        &self,
        table_ids: &IndexMap<String, TableId>,
        schema_name: &str,
        sql_schema: &mut SqlSchema,
        flavour: &Flavour,
    ) -> DescriberResult<()> {
        // We alias all the columns because MySQL column names are case-insensitive in queries, but the
        // information schema column names became upper-case in MySQL 8, causing the code fetching
        // the result values by column name below to fail.
        let sql_geometry_srid_column = if self.supports_srid_constraints() {
            "srs_id"
        } else {
            "NULL"
        };

        let sql = format!(
            "
            SELECT
                column_name column_name,
                data_type data_type,
                column_type full_data_type,
                character_maximum_length character_maximum_length,
                numeric_precision numeric_precision,
                numeric_scale numeric_scale,
                {sql_geometry_srid_column} geometry_srid,
                datetime_precision datetime_precision,
                column_default column_default,
                is_nullable is_nullable,
                extra extra,
                table_name table_name,
                NULLIF(column_comment, '') AS column_comment
            FROM information_schema.columns
            WHERE table_schema = ?
            ORDER BY ordinal_position
        "
        );

        let mut table_defaults = Vec::new();
        let mut view_defaults = Vec::new();
        let rows = self.conn.query_raw(&sql, &[schema_name.into()]).await?;

        for col in rows {
            trace!("Got column: {col:?}");
            let table_name = col.get_expect_string("table_name");

            let table_id = table_ids.get(table_name.as_str());
            let view_id = sql_schema.view_walker(table_name.as_str());

            let container_id = match (table_id, view_id) {
                (Some(id), _) => Either::Left(id),
                (_, Some(v_walker)) => Either::Right(v_walker.id),
                (None, None) => continue, // we only care about columns in tables we have access to
            };

            let name = col.get_expect_string("column_name");
            let data_type = col.get_expect_string("data_type");
            let full_data_type = col.get_expect_string("full_data_type");

            let is_nullable = col.get_expect_string("is_nullable").to_lowercase();
            let is_required = match is_nullable.as_ref() {
                "no" => true,
                "yes" => false,
                x => panic!("unrecognized is_nullable variant '{x}'"),
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
            let geometry_srid = col.get_u32("geometry_srid");

            let precision = Precision {
                character_maximum_length,
                numeric_precision,
                numeric_scale,
                time_precision,
            };

            let default_value = col.get("column_default");

            let tpe = Self::get_column_type(
                (&table_name, &name),
                (&data_type, &full_data_type),
                precision,
                geometry_srid,
                arity,
                default_value,
                sql_schema,
            );
            let extra = col.get_expect_string("extra").to_lowercase();
            let auto_increment = matches!(extra.as_str(), "auto_increment");

            let default = match default_value {
                None => None,
                Some(param_value) => match param_value.to_string() {
                    None => None,
                    Some(x) if x == "NULL" => None,
                    Some(default_string) => {
                        let default_generated = matches!(extra.as_str(), "default_generated");
                        let maria_db = matches!(flavour, Flavour::MariaDb);
                        let default_expression = default_generated || maria_db;

                        Some(match &tpe.family {
                            ColumnTypeFamily::Int => match Self::parse_int(&default_string) {
                                Some(int_value) => DefaultValue::value(int_value),
                                None if default_expression => Self::dbgenerated_expression(&default_string),
                                None => DefaultValue::db_generated(default_string),
                            },
                            ColumnTypeFamily::BigInt => match Self::parse_big_int(&default_string) {
                                Some(int_value) => DefaultValue::value(int_value),
                                None if default_expression => Self::dbgenerated_expression(&default_string),
                                None => DefaultValue::db_generated(default_string),
                            },
                            ColumnTypeFamily::Float => match Self::parse_float(&default_string) {
                                Some(float_value) => DefaultValue::value(float_value),
                                None if default_expression => Self::dbgenerated_expression(&default_string),
                                None => DefaultValue::db_generated(default_string),
                            },
                            ColumnTypeFamily::Decimal => match Self::parse_float(&default_string) {
                                Some(float_value) => DefaultValue::value(float_value),
                                None if default_expression => Self::dbgenerated_expression(&default_string),
                                None => DefaultValue::db_generated(default_string),
                            },
                            ColumnTypeFamily::Boolean => match Self::parse_int(&default_string) {
                                Some(PrismaValue::Int(1)) => DefaultValue::value(true),
                                Some(PrismaValue::Int(0)) => DefaultValue::value(false),
                                _ if default_expression => Self::dbgenerated_expression(&default_string),
                                _ => DefaultValue::db_generated(default_string),
                            },
                            ColumnTypeFamily::String => {
                                // See https://dev.mysql.com/doc/refman/8.0/en/information-schema-columns-table.html
                                // and https://mariadb.com/kb/en/information-schema-columns-table/
                                if default_generated
                                    || (maria_db && !matches!(default_string.chars().next(), Some('\'')))
                                {
                                    Self::dbgenerated_expression(&default_string)
                                } else {
                                    DefaultValue::value(PrismaValue::String(Self::unescape_and_unquote_default_string(
                                        default_string,
                                        flavour,
                                    )))
                                }
                            }
                            ColumnTypeFamily::DateTime => match Self::default_is_current_timestamp(&default_string) {
                                true => DefaultValue::now(),
                                _ if default_expression => Self::dbgenerated_expression(&default_string),
                                _ if DEFAULT_QUOTES.is_match(&default_string) => {
                                    DefaultValue::db_generated(default_string)
                                }
                                _ => DefaultValue::db_generated(format!("'{default_string}'")),
                            },
                            ColumnTypeFamily::Binary => match default_expression {
                                true => Self::dbgenerated_expression(&default_string),
                                false => DefaultValue::db_generated(default_string),
                            },
                            ColumnTypeFamily::Json => match default_expression {
                                true => Self::dbgenerated_expression(&default_string),
                                false => DefaultValue::db_generated(default_string),
                            },
                            ColumnTypeFamily::Geometry => match default_expression {
                                true => Self::dbgenerated_expression(&default_string),
                                false => DefaultValue::db_generated(default_string),
                            },
                            ColumnTypeFamily::Uuid => match default_expression {
                                true => Self::dbgenerated_expression(&default_string),
                                false => DefaultValue::db_generated(default_string),
                            },
                            ColumnTypeFamily::Enum(_) => {
                                if default_generated
                                    || (maria_db && !matches!(default_string.chars().next(), Some('\'')))
                                {
                                    Self::dbgenerated_expression(&default_string)
                                } else {
                                    DefaultValue::value(PrismaValue::Enum(Self::unquote_string(
                                        &default_string
                                            .replace("_utf8mb4", "")
                                            .replace("\\\'", "")
                                            .replace("''", "'"),
                                    )))
                                }
                            }
                            ColumnTypeFamily::Unsupported(_) => match default_expression {
                                true => Self::dbgenerated_expression(&default_string),
                                false => DefaultValue::db_generated(default_string),
                            },
                        })
                    }
                },
            };

            match container_id {
                Either::Left(table_id) => {
                    table_defaults.push((table_id, default));
                }
                Either::Right(view_id) => {
                    view_defaults.push((view_id, default));
                }
            }

            let description = col.get_string("column_comment");

            let col = Column {
                name,
                tpe,
                auto_increment,
                description,
            };

            match container_id {
                Either::Left(table_id) => {
                    sql_schema.table_columns.push((*table_id, col));
                }
                Either::Right(view_id) => {
                    sql_schema.view_columns.push((view_id, col));
                }
            }
        }

        sql_schema.table_columns.sort_by_key(|(table_id, _)| *table_id);
        sql_schema.view_columns.sort_by_key(|(view_id, _)| *view_id);

        table_defaults.sort_by_key(|(table_id, _)| *table_id);
        view_defaults.sort_by_key(|(view_id, _)| *view_id);

        for (i, (_, default)) in table_defaults.into_iter().enumerate() {
            if let Some(default) = default {
                sql_schema.push_table_default_value(TableColumnId(i as u32), default);
            }
        }

        for (i, (_, default)) in view_defaults.into_iter().enumerate() {
            if let Some(default) = default {
                sql_schema.push_view_default_value(ViewColumnId(i as u32), default);
            }
        }

        Ok(())
    }

    fn dbgenerated_expression(default_string: &str) -> DefaultValue {
        if matches!(default_string.chars().next(), Some('(')) {
            DefaultValue::db_generated(default_string.to_owned())
        } else {
            let mut introspected_default = String::with_capacity(default_string.len());
            introspected_default.push('(');
            introspected_default.push_str(default_string);
            introspected_default.push(')');
            DefaultValue::db_generated(introspected_default)
        }
    }

    fn get_column_type(
        (table, column_name): (&str, &str),
        (data_type, full_data_type): (&str, &str),
        precision: Precision,
        geometry_srid: Option<u32>,
        arity: ColumnArity,
        default: Option<&Value<'_>>,
        sql_schema: &mut SqlSchema,
    ) -> ColumnType {
        static UNSIGNEDNESS_RE: Lazy<Regex> = Lazy::new(|| Regex::new("(?i)unsigned$").unwrap());
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
            "enum" => {
                let enum_name = format!("{table}_{column_name}");
                let enum_id = sql_schema.push_enum(Default::default(), enum_name, None);
                push_enum_variants(full_data_type, enum_id, sql_schema);
                (ColumnTypeFamily::Enum(enum_id), None)
            }
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
            "geometry" => (ColumnTypeFamily::Geometry, Some(MySqlType::Geometry(geometry_srid))),
            "point" => (ColumnTypeFamily::Geometry, Some(MySqlType::Point(geometry_srid))),
            "linestring" => (ColumnTypeFamily::Geometry, Some(MySqlType::LineString(geometry_srid))),
            "polygon" => (ColumnTypeFamily::Geometry, Some(MySqlType::Polygon(geometry_srid))),
            "multipoint" => (ColumnTypeFamily::Geometry, Some(MySqlType::MultiPoint(geometry_srid))),
            "multilinestring" => (
                ColumnTypeFamily::Geometry,
                Some(MySqlType::MultiLineString(geometry_srid)),
            ),
            "multipolygon" => (ColumnTypeFamily::Geometry, Some(MySqlType::MultiPolygon(geometry_srid))),
            "geometrycollection" | "geomcollection" => (
                ColumnTypeFamily::Geometry,
                Some(MySqlType::GeometryCollection(geometry_srid)),
            ),
            _ => (ColumnTypeFamily::Unsupported(full_data_type.into()), None),
        };

        ColumnType {
            full_data_type: full_data_type.to_owned(),
            family,
            arity,
            native_type: native_type.map(NativeTypeInstance::new::<MySqlType>),
        }
    }

    /// Return the constraints that are not primary keys, foreign keys, or unique keys, fulltext, or spacial.
    /// Namely, this currently just returns CHECK constraints.
    async fn get_constraints(
        &self,
        table_names: &IndexMap<String, TableId>,
        sql_schema: &mut SqlSchema,
    ) -> DescriberResult<()> {
        // Only MySQL 8.0.16 and above supports check constraints and has the CHECK_CONSTRAINTS table we can query.
        if !self.supports_check_constraints() {
            return Ok(());
        }

        // Note: in MySQL, columns must be re-aliased, otherwise their casing would be inconsistent (and uppercased).
        let sql = include_str!("mysql/constraints_query.sql");

        let rows = self.conn.query_raw(sql, &[]).await?;

        for row in rows {
            let table_name = row.get_expect_string("table_name");
            let constraint_name = row.get_expect_string("constraint_name");
            let constraint_type = row.get_expect_string("constraint_type");

            let table_id = match table_names.get(&table_name) {
                Some(id) => *id,
                None => continue,
            };

            if constraint_type.as_str() == "check" {
                sql_schema.check_constraints.push((table_id, constraint_name));
            }
        }

        sql_schema.check_constraints.sort_by_key(|(id, _)| *id);

        Ok(())
    }

    fn extract_precision(input: &str) -> Option<u32> {
        static RE: Lazy<Regex> = Lazy::new(|| Regex::new(r".*\(([1-9])\)").unwrap());
        RE.captures(input)
            .and_then(|cap| cap.get(1).map(|precision| precision.as_str().parse::<u32>().unwrap()))
    }

    // See https://dev.mysql.com/doc/refman/8.0/en/string-literals.html
    //
    // In addition, MariaDB will return string literals with the quotes and extra backslashes around
    // control characters like `\n`.
    fn unescape_and_unquote_default_string(default: String, flavour: &Flavour) -> String {
        static MYSQL_ESCAPING_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"\\('|\\[^\\])|'(')").unwrap());
        static MARIADB_NEWLINE_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"\\n").unwrap());
        static MARIADB_DEFAULT_QUOTE_UNESCAPE_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r#"'(.*)'"#).unwrap());

        let maybe_unquoted: Cow<'_, str> = if matches!(flavour, Flavour::MariaDb) {
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
            Lazy::new(|| Regex::new(r"(?i)^current_timestamp(\([0-9]*\))?$").unwrap());

        MYSQL_CURRENT_TIMESTAMP_RE.is_match(default_str)
    }

    /// Tests whether the current database supports check constraints
    fn supports_check_constraints(&self) -> bool {
        self.circumstances.contains(Circumstances::CheckConstraints)
    }

    /// Tests whether the current database supports geometry SRID constraints
    fn supports_srid_constraints(&self) -> bool {
        // Only MySQL 8 and above supports geometry SRIDs constraints
        !self
            .circumstances
            .intersects(Circumstances::MySql56 | Circumstances::MySql57 | Circumstances::MariaDb)
    }
}

async fn push_foreign_keys(
    schema_name: &str,
    table_ids: &IndexMap<String, TableId>,
    sql_schema: &mut SqlSchema,
    conn: &dyn Queryable,
) -> DescriberResult<()> {
    // We alias all the columns because MySQL column names are case-insensitive in queries, but
    // the information schema column names became upper-case in MySQL 8, causing the code
    // fetching the result values by column name below to fail.
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
                BINARY kcu.constraint_name = BINARY rc.constraint_name
            WHERE
                BINARY kcu.table_schema = ?
                AND BINARY rc.constraint_schema = ?
                AND kcu.referenced_column_name IS NOT NULL

            ORDER BY
                BINARY kcu.table_schema,
                BINARY kcu.table_name,
                BINARY kcu.constraint_name,
                kcu.ordinal_position
        ";

    fn get_ids(
        row: &ResultRow,
        table_ids: &IndexMap<String, TableId>,
        sql_schema: &SqlSchema,
    ) -> Option<(TableId, TableColumnId, TableId, TableColumnId)> {
        let table_name = row.get_expect_string("table_name");
        let column_name = row.get_expect_string("column_name");
        let referenced_table_name = row.get_expect_string("referenced_table_name");
        let referenced_column_name = row.get_expect_string("referenced_column_name");

        let table_id = *table_ids.get(&table_name)?;
        let referenced_table_id = *table_ids.get(&referenced_table_name)?;
        let column_id = sql_schema.walk(table_id).column(&column_name)?.id;
        let referenced_column_id = sql_schema.walk(referenced_table_id).column(&referenced_column_name)?.id;

        Some((table_id, column_id, referenced_table_id, referenced_column_id))
    }

    let result_set = conn.query_raw(sql, &[schema_name.into(), schema_name.into()]).await?;
    let mut current_fk: Option<(TableId, String, ForeignKeyId)> = None;

    for row in result_set.into_iter() {
        trace!("Got description FK row {row:#?}");
        let (table_id, column_id, referenced_table_id, referenced_column_id) =
            if let Some(ids) = get_ids(&row, table_ids, sql_schema) {
                ids
            } else {
                continue;
            };
        let constraint_name = row.get_expect_string("constraint_name");
        let on_delete_action = match row.get_expect_string("delete_rule").to_lowercase().as_str() {
            "cascade" => ForeignKeyAction::Cascade,
            "set null" => ForeignKeyAction::SetNull,
            "set default" => ForeignKeyAction::SetDefault,
            "restrict" => ForeignKeyAction::Restrict,
            "no action" => ForeignKeyAction::NoAction,
            s => panic!("Unrecognized on delete action '{s}'"),
        };
        let on_update_action = match row.get_expect_string("update_rule").to_lowercase().as_str() {
            "cascade" => ForeignKeyAction::Cascade,
            "set null" => ForeignKeyAction::SetNull,
            "set default" => ForeignKeyAction::SetDefault,
            "restrict" => ForeignKeyAction::Restrict,
            "no action" => ForeignKeyAction::NoAction,
            s => panic!("Unrecognized on update action '{s}'"),
        };

        match &current_fk {
            Some((cur_table_id, cur_constraint_name, _))
                if *cur_table_id == table_id && *cur_constraint_name == constraint_name => {}
            None | Some(_) => {
                let fkid = sql_schema.push_foreign_key(
                    Some(constraint_name.clone()),
                    [table_id, referenced_table_id],
                    [on_delete_action, on_update_action],
                );

                current_fk = Some((table_id, constraint_name, fkid));
            }
        }

        if let Some((_, _, fkid)) = current_fk {
            sql_schema.push_foreign_key_column(fkid, [column_id, referenced_column_id]);
        }
    }

    Ok(())
}

fn push_enum_variants(full_data_type: &str, enum_id: EnumId, sql_schema: &mut SqlSchema) {
    let len = &full_data_type.len() - 1;
    // full_data_type for enum columns follows the pattern "enum('a','b')"
    let vals = &full_data_type[5..len];
    for variant in vals.split(',').map(unquote_string) {
        sql_schema.push_enum_variant(enum_id, variant.replace("''", "'"));
    }
}
