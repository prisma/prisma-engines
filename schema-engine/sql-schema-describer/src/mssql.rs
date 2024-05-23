//! SQL server schema description.

use crate::{
    getters::Getter, ids::*, parsers::Parser, Column, ColumnArity, ColumnType, ColumnTypeFamily, DefaultValue,
    DescriberError, DescriberErrorKind, DescriberResult, ForeignKeyAction, IndexColumn, Procedure, SQLSortOrder,
    SqlMetadata, SqlSchema, UserDefinedType, View,
};
use either::Either;
use enumflags2::BitFlags;
use indexmap::IndexMap;
use indoc::indoc;
use once_cell::sync::Lazy;
use prisma_value::PrismaValue;
use psl::{
    builtin_connectors::{MsSqlType, MsSqlTypeParameter},
    datamodel_connector::NativeTypeInstance,
};
use quaint::prelude::Queryable;
use regex::Regex;
use std::{any::type_name, borrow::Cow, collections::HashMap, convert::TryInto};

/// Matches a default value in the schema, that is not a string.
///
/// Examples:
///
/// ```ignore
/// ((1))
/// ```
///
/// ```ignore
/// ((1.123))
/// ```
///
/// ```ignore
/// ((true))
/// ```
static DEFAULT_NON_STRING: Lazy<Regex> = Lazy::new(|| Regex::new(r"\(\((.*)\)\)").unwrap());

/// Matches a default value in the schema, that is a string.
///
/// Example:
///
/// ```ignore
/// ('this is a test')
/// ```
static DEFAULT_STRING: Lazy<Regex> = Lazy::new(|| Regex::new(r"\('([\S\s]*)'\)").unwrap());

/// Matches a database-generated value in the schema.
///
/// Example:
///
/// ```ignore
/// (current_timestamp)
/// ```
static DEFAULT_DB_GEN: Lazy<Regex> = Lazy::new(|| Regex::new(r"\((.*)\)").unwrap());

/// Matches a shared default constraint (which we will skip).
///
/// example:
///
/// ```ignore
/// CREATE DEFAULT catcat AS 'musti';
/// ```
static DEFAULT_SHARED_CONSTRAINT: Lazy<Regex> = Lazy::new(|| Regex::new(r"CREATE DEFAULT (.*)").unwrap());

pub struct SqlSchemaDescriber<'a> {
    conn: &'a dyn Queryable,
}

#[derive(Default)]
pub struct MssqlSchemaExt {
    pub index_bits: HashMap<IndexId, BitFlags<IndexBits>>,
}

#[enumflags2::bitflags]
#[repr(u8)]
#[derive(Clone, Copy)]
pub enum IndexBits {
    Clustered = 0b1,
    Constraint = 0b10,
}

impl MssqlSchemaExt {
    pub fn index_is_clustered(&self, id: IndexId) -> bool {
        self.index_bits
            .get(&id)
            .map(|b| b.contains(IndexBits::Clustered))
            .unwrap_or(false)
    }

    pub fn index_is_a_constraint(&self, id: IndexId) -> bool {
        self.index_bits
            .get(&id)
            .map(|b| b.contains(IndexBits::Constraint))
            .unwrap_or(false)
    }
}

impl std::fmt::Debug for SqlSchemaDescriber<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct(type_name::<SqlSchemaDescriber<'_>>()).finish()
    }
}

#[async_trait::async_trait]
impl super::SqlSchemaDescriberBackend for SqlSchemaDescriber<'_> {
    async fn list_databases(&self) -> DescriberResult<Vec<String>> {
        Ok(self.get_databases().await?)
    }

    async fn get_metadata(&self, schema: &str) -> DescriberResult<SqlMetadata> {
        let mut sql_schema = SqlSchema::default();

        self.get_namespaces(&mut sql_schema, &[schema]).await?;

        let table_count = self.get_table_names(&mut sql_schema).await?.len();
        let size_in_bytes = self.get_size(schema).await?;

        Ok(SqlMetadata {
            table_count,
            size_in_bytes,
        })
    }

    async fn describe(&self, schemas: &[&str]) -> DescriberResult<SqlSchema> {
        let mut sql_schema = SqlSchema::default();
        let mut mssql_ext = MssqlSchemaExt::default();

        self.get_namespaces(&mut sql_schema, schemas).await?;

        let table_names = self.get_table_names(&mut sql_schema).await?;

        self.get_views(&mut sql_schema).await?;
        self.get_columns(&mut sql_schema).await?;
        self.get_all_indices(&mut mssql_ext, &table_names, &mut sql_schema)
            .await?;
        self.get_foreign_keys(&table_names, &mut sql_schema).await?;

        self.get_procedures(&mut sql_schema).await?;
        self.get_user_defined_types(&mut sql_schema).await?;

        sql_schema.connector_data = crate::connector_data::ConnectorData {
            data: Some(Box::new(mssql_ext)),
        };

        Ok(sql_schema)
    }

    async fn version(&self) -> DescriberResult<Option<String>> {
        Ok(self.conn.version().await?)
    }
}

impl Parser for SqlSchemaDescriber<'_> {}

impl<'a> SqlSchemaDescriber<'a> {
    pub fn new(conn: &'a dyn Queryable) -> Self {
        Self { conn }
    }

    async fn get_databases(&self) -> DescriberResult<Vec<String>> {
        let sql = "SELECT name FROM sys.schemas";
        let rows = self.conn.query_raw(sql, &[]).await?;
        Ok(rows.into_iter().map(|row| row.get_expect_string("name")).collect())
    }

    async fn get_procedures(&self, sql_schema: &mut SqlSchema) -> DescriberResult<()> {
        let sql = r#"
            SELECT
                name,
                OBJECT_DEFINITION(object_id) AS definition,
                SCHEMA_NAME(schema_id) AS namespace
            FROM sys.objects
            WHERE is_ms_shipped = 0 AND type = 'P'
            ORDER BY name;
        "#;

        let rows = self.conn.query_raw(sql, &[]).await?;
        let mut procedures = Vec::with_capacity(rows.len());

        for row in rows.into_iter() {
            let namespace_id = match sql_schema.get_namespace_id(&row.get_expect_string("namespace")) {
                Some(id) => id,
                None => continue,
            };

            procedures.push(Procedure {
                namespace_id,
                name: row.get_expect_string("name"),
                definition: row.get_string("definition"),
            });
        }

        sql_schema.procedures = procedures;

        Ok(())
    }

    async fn get_table_names(
        &self,
        sql_schema: &mut SqlSchema,
    ) -> DescriberResult<IndexMap<(String, String), TableId>> {
        let select = r#"
            SELECT
                tbl.name AS table_name,
                SCHEMA_NAME(tbl.schema_id) AS namespace
            FROM sys.tables tbl
            WHERE tbl.is_ms_shipped = 0 AND tbl.type = 'U'
            ORDER BY tbl.name;
        "#;

        let rows = self.conn.query_raw(select, &[]).await?;

        let names = rows
            .into_iter()
            .filter(|row| sql_schema.namespaces.contains(&row.get_expect_string("namespace")))
            .map(|row| (row.get_expect_string("table_name"), row.get_expect_string("namespace")))
            .collect::<Vec<_>>();

        let mut map = IndexMap::new();

        for (table_name, namespace) in names {
            let namespace_id = match sql_schema.get_namespace_id(&namespace) {
                Some(id) => id,
                None => continue,
            };

            let cloned_name = table_name.clone();
            let id = sql_schema.push_table(table_name, namespace_id, None);
            map.insert((namespace, cloned_name), id);
        }

        Ok(map)
    }

    async fn get_size(&self, schema: &str) -> DescriberResult<usize> {
        let sql = indoc! {r#"
            SELECT
                SUM(a.total_pages) * 8000 AS size
            FROM
                sys.tables t
            INNER JOIN
                sys.partitions p ON t.object_id = p.object_id
            INNER JOIN
                sys.allocation_units a ON p.partition_id = a.container_id
            WHERE SCHEMA_NAME(t.schema_id) = @P1
                AND t.is_ms_shipped = 0
            GROUP BY
                t.schema_id
            ORDER BY
                size DESC;
        "#};

        let rows = self.conn.query_raw(sql, &[schema.into()]).await?;

        let size: i64 = rows
            .into_single()
            .map(|row| row.get("size").and_then(|x| x.as_integer()).unwrap_or(0))
            .unwrap_or(0);

        Ok(size
            .try_into()
            .expect("Invariant violation: size is not a valid usize value."))
    }

    async fn get_columns(&self, sql_schema: &mut SqlSchema) -> DescriberResult<()> {
        let sql = indoc! {r#"
            SELECT c.name                                                       AS column_name,
                CASE typ.is_assembly_type
                        WHEN 1 THEN TYPE_NAME(c.user_type_id)
                        ELSE TYPE_NAME(c.system_type_id)
                END                                                             AS data_type,
                COLUMNPROPERTY(c.object_id, c.name, 'charmaxlen')               AS character_maximum_length,
                OBJECT_DEFINITION(c.default_object_id)                          AS column_default,
                c.is_nullable                                                   AS is_nullable,
                COLUMNPROPERTY(c.object_id, c.name, 'IsIdentity')               AS is_identity,
                OBJECT_NAME(c.object_id)                                        AS table_name,
                OBJECT_NAME(c.default_object_id)                                AS constraint_name,
                convert(tinyint, CASE
                    WHEN c.system_type_id IN (48, 52, 56, 59, 60, 62, 106, 108, 122, 127) THEN c.precision
                    END) AS numeric_precision,
                convert(int, CASE
                    WHEN c.system_type_id IN (40, 41, 42, 43, 58, 61) THEN NULL
                    ELSE ODBCSCALE(c.system_type_id, c.scale) END) AS numeric_scale,
                OBJECT_SCHEMA_NAME(c.object_id) AS namespace
            FROM sys.columns c
                    INNER JOIN sys.objects obj ON c.object_id = obj.object_id
                    INNER JOIN sys.types typ ON c.user_type_id = typ.user_type_id
            WHERE obj.is_ms_shipped = 0
            ORDER BY table_name, COLUMNPROPERTY(c.object_id, c.name, 'ordinal');
        "#};

        let rows = self.conn.query_raw(sql, &[]).await?;
        let mut table_defaults = Vec::new();
        let mut view_defaults = Vec::new();

        for col in rows {
            let namespace = col.get_expect_string("namespace");
            let table_name = col.get_expect_string("table_name");

            let table_id = sql_schema.table_walker_ns(&namespace, &table_name);
            let view_id = sql_schema.view_walker_ns(&namespace, &table_name);

            let container_id = match (table_id, view_id) {
                (Some(t_walker), _) => Either::Left(t_walker.id),
                (_, Some(v_walker)) => Either::Right(v_walker.id),
                (None, None) => continue, // we only care about columns in tables we have access to
            };

            let name = col.get_expect_string("column_name");
            let data_type = col.get_expect_string("data_type");
            let character_maximum_length = col.get_i64("character_maximum_length");

            let numeric_precision = col.get_u32("numeric_precision");
            let numeric_scale = col.get_u32("numeric_scale");
            let is_nullable = &col.get_expect_bool("is_nullable");

            let arity = if !is_nullable {
                ColumnArity::Required
            } else {
                ColumnArity::Nullable
            };

            let tpe = self.get_column_type(
                &data_type,
                character_maximum_length,
                numeric_precision,
                numeric_scale,
                arity,
            );

            let auto_increment = col.get_expect_bool("is_identity");

            let default = match col.get("column_default") {
                None => None,
                Some(param_value) => match param_value.to_string() {
                    None => None,
                    Some(x) if x == "(NULL)" => None,
                    Some(x) if DEFAULT_SHARED_CONSTRAINT.is_match(&x) => None,
                    Some(default_string) => {
                        let default_string = DEFAULT_NON_STRING
                            .captures_iter(&default_string)
                            .next()
                            .or_else(|| DEFAULT_STRING.captures_iter(&default_string).next())
                            .or_else(|| DEFAULT_DB_GEN.captures_iter(&default_string).next())
                            .map(|cap| cap[1].to_string())
                            .ok_or_else(|| format!("Couldn't parse default value: `{default_string}`"))
                            .unwrap();

                        let mut default = match tpe.family {
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
                                Some(PrismaValue::Int(1)) => DefaultValue::value(PrismaValue::Boolean(true)),
                                Some(PrismaValue::Int(0)) => DefaultValue::value(PrismaValue::Boolean(false)),
                                _ => DefaultValue::db_generated(default_string),
                            },
                            ColumnTypeFamily::String => DefaultValue::value(default_string.replace("''", "'")),
                            //todo check other now() definitions
                            ColumnTypeFamily::DateTime => match default_string.as_str() {
                                "getdate()" => DefaultValue::now(),
                                _ => DefaultValue::db_generated(default_string),
                            },
                            ColumnTypeFamily::Binary => DefaultValue::db_generated(default_string),
                            ColumnTypeFamily::Json => DefaultValue::db_generated(default_string),
                            ColumnTypeFamily::Uuid => DefaultValue::db_generated(default_string),
                            ColumnTypeFamily::Unsupported(_) => DefaultValue::db_generated(default_string),
                            ColumnTypeFamily::Enum(_) => unreachable!("No enums in MSSQL"),
                        };

                        if let Some(name) = col.get_string("constraint_name") {
                            default.set_constraint_name(name);
                        }

                        Some(default)
                    }
                },
            };

            let column = Column {
                name,
                tpe,
                auto_increment,
                description: None,
            };

            match container_id {
                Either::Left(table_id) => {
                    table_defaults.push((table_id, default));
                    sql_schema.push_table_column(table_id, column);
                }
                Either::Right(view_id) => {
                    view_defaults.push((view_id, default));
                    sql_schema.push_view_column(view_id, column);
                }
            }
        }

        sql_schema.table_columns.sort_by_key(|(table_id, _)| *table_id);
        sql_schema.view_columns.sort_by_key(|(table_id, _)| *table_id);

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

    async fn get_all_indices(
        &self,
        mssql_ext: &mut MssqlSchemaExt,
        table_ids: &IndexMap<(String, String), TableId>,
        sql_schema: &mut SqlSchema,
    ) -> DescriberResult<()> {
        let sql = indoc! {r#"
            SELECT DISTINCT
                ind.name AS index_name,
                ind.is_unique AS is_unique,
                ind.is_unique_constraint AS is_unique_constraint,
                ind.is_primary_key AS is_primary_key,
                ind.type_desc AS clustering,
                col.name AS column_name,
                ic.key_ordinal AS seq_in_index,
                ic.is_descending_key AS is_descending,
                t.name AS table_name,
                SCHEMA_NAME(t.schema_id) AS namespace
            FROM
                sys.indexes ind
            INNER JOIN sys.index_columns ic
                ON ind.object_id = ic.object_id AND ind.index_id = ic.index_id
            INNER JOIN sys.columns col
                ON ic.object_id = col.object_id AND ic.column_id = col.column_id
            INNER JOIN
                sys.tables t ON ind.object_id = t.object_id
            WHERE t.is_ms_shipped = 0
                -- https://docs.microsoft.com/en-us/sql/relational-databases/system-catalog-views/sys-index-columns-transact-sql?view=sql-server-ver16
                AND ic.key_ordinal != 0
                AND ind.filter_definition IS NULL
                AND ind.name IS NOT NULL
                AND ind.type_desc IN (
                    'CLUSTERED',
                    'NONCLUSTERED',
                    'CLUSTERED COLUMNSTORE',
                    'NONCLUSTERED COLUMNSTORE'
                )
            ORDER BY table_name, index_name, seq_in_index
        "#};

        let rows = self.conn.query_raw(sql, &[]).await?;
        let mut current_index: Option<IndexId> = None;

        for row in rows {
            let namespace = row.get_expect_string("namespace");
            let table_name = row.get_expect_string("table_name");

            let table_id: TableId = match table_ids.get(&(namespace, table_name)) {
                Some(id) => *id,
                None => continue,
            };

            let index_name = row.get_expect_string("index_name");

            let sort_order = match row.get_expect_bool("is_descending") {
                true => SQLSortOrder::Desc,
                false => SQLSortOrder::Asc,
            };

            let clustered = row.get_expect_string("clustering").starts_with("CLUSTERED");

            let column_name = row.get_expect_string("column_name");
            let column_id = if let Some(col) = sql_schema.walk(table_id).column(&column_name) {
                col.id
            } else {
                continue;
            };
            // Multi-column indices will return more than one row (with different column_name values).
            // We cannot assume that one row corresponds to one index.
            let seq_in_index = row.get_expect_i64("seq_in_index");
            let is_unique = row.get_expect_bool("is_unique");
            let is_unique_constraint = row.get_expect_bool("is_unique_constraint");
            let is_pk = row.get_expect_bool("is_primary_key");

            if seq_in_index == 1 {
                // new index!
                let id = if is_pk {
                    sql_schema.push_primary_key(table_id, index_name)
                } else if is_unique {
                    sql_schema.push_unique_constraint(table_id, index_name)
                } else {
                    sql_schema.push_index(table_id, index_name)
                };

                let mut bits = BitFlags::empty();
                if clustered {
                    bits |= IndexBits::Clustered;
                }
                if is_unique_constraint {
                    bits |= IndexBits::Constraint;
                }
                mssql_ext.index_bits.insert(id, bits);

                current_index = Some(id);
            };

            sql_schema.push_index_column(IndexColumn {
                index_id: current_index.unwrap(),
                column_id,
                sort_order: Some(sort_order),
                length: None,
            });
        }

        Ok(())
    }

    async fn get_namespaces(&self, sql_schema: &mut SqlSchema, namespaces: &[&str]) -> DescriberResult<()> {
        let rows = self
            .conn
            .query_raw("SELECT name FROM sys.schemas ORDER BY name", &[])
            .await?;

        let names = rows
            .into_iter()
            .map(|row| row.get_expect_string("name"))
            .filter(|name| namespaces.contains(&name.as_str()));

        for name in names {
            sql_schema.push_namespace(name);
        }

        Ok(())
    }

    async fn get_views(&self, sql_schema: &mut SqlSchema) -> DescriberResult<()> {
        let sql = indoc! {r#"
            SELECT
                name AS view_name,
                OBJECT_DEFINITION(object_id) AS view_sql,
                SCHEMA_NAME(schema_id) AS namespace
            FROM sys.views
            WHERE is_ms_shipped = 0
        "#};

        let result_set = self.conn.query_raw(sql, &[]).await?;
        let mut views = Vec::with_capacity(result_set.len());

        for row in result_set.into_iter() {
            let namespace_id = match sql_schema.get_namespace_id(&row.get_expect_string("namespace")) {
                Some(id) => id,
                None => continue,
            };

            views.push(View {
                namespace_id,
                name: row.get_expect_string("view_name"),
                definition: row.get_string("view_sql"),
                description: None,
            })
        }

        sql_schema.views = views;

        Ok(())
    }

    async fn get_user_defined_types(&self, sql_schema: &mut SqlSchema) -> DescriberResult<()> {
        let sql = indoc! {r#"
            SELECT
                udt.name AS user_type_name,
                systyp.name AS system_type_name,
                CONVERT(SMALLINT,
                        CASE
                            -- nchar + nvarchar are double size
                            WHEN udt.system_type_id IN (231, 239) AND udt.max_length = -1 THEN -1
                            -- nchar + nvarchar are double size
                            WHEN udt.system_type_id IN (231, 239) THEN udt.max_length / 2.0
                            -- varbinary, varchar, binary and char
                            WHEN udt.system_type_id IN (165, 167, 173, 175) THEN udt.max_length
                            ELSE null
                            END) AS max_length,
                CONVERT(tinyint,
                        CASE
                            -- numeric, decimal
                            WHEN udt.system_type_id IN (106, 108) THEN udt.precision
                            ELSE null
                            END) AS precision,
                CONVERT(tinyint,
                        CASE
                            -- numeric, decimal
                            WHEN udt.system_type_id IN (106, 108) THEN udt.scale
                            ELSE null
                            END) AS scale,
                SCHEMA_NAME(udt.schema_id) AS namespace
            FROM sys.types udt
                    LEFT JOIN sys.types systyp
                            ON udt.system_type_id = systyp.user_type_id
            WHERE udt.is_user_defined = 1
        "#};

        let result_set = self.conn.query_raw(sql, &[]).await?;

        sql_schema.user_defined_types = result_set
            .into_iter()
            .flat_map(|row| {
                let namespace_id = match sql_schema.get_namespace_id(&row.get_expect_string("namespace")) {
                    Some(id) => id,
                    None => return None,
                };

                let name = row.get_expect_string("user_type_name");
                let max_length = row.get_i64("max_length");
                let precision = row.get_u32("precision");
                let scale = row.get_u32("scale");

                let definition = row
                    .get_string("system_type_name")
                    .map(|name| match (max_length, precision, scale) {
                        (Some(-1), _, _) => format!("{name}(max)"),
                        (Some(len), _, _) => format!("{name}({len})"),
                        (_, Some(p), Some(s)) => format!("{name}({p},{s})"),
                        _ => name,
                    });

                Some(UserDefinedType {
                    namespace_id,
                    name,
                    definition,
                })
            })
            .collect();

        Ok(())
    }

    async fn get_foreign_keys(
        &self,
        table_ids: &IndexMap<(String, String), TableId>,
        sql_schema: &mut SqlSchema,
    ) -> DescriberResult<()> {
        let sql = indoc! {r#"
            SELECT OBJECT_NAME(fkc.constraint_object_id) AS constraint_name,
                parent_table.name                        AS table_name,
                referenced_table.name                    AS referenced_table_name,
                SCHEMA_NAME(referenced_table.schema_id)  AS referenced_schema_name,
                parent_column.name                       AS column_name,
                referenced_column.name                   AS referenced_column_name,
                fk.delete_referential_action             AS delete_referential_action,
                fk.update_referential_action             AS update_referential_action,
                fkc.constraint_column_id                 AS ordinal_position,
                OBJECT_SCHEMA_NAME(fkc.parent_object_id) AS schema_name
            FROM sys.foreign_key_columns AS fkc
                    INNER JOIN sys.tables AS parent_table
                                ON fkc.parent_object_id = parent_table.object_id
                    INNER JOIN sys.tables AS referenced_table
                                ON fkc.referenced_object_id = referenced_table.object_id
                    INNER JOIN sys.columns AS parent_column
                                ON fkc.parent_object_id = parent_column.object_id
                                    AND fkc.parent_column_id = parent_column.column_id
                    INNER JOIN sys.columns AS referenced_column
                                ON fkc.referenced_object_id = referenced_column.object_id
                                    AND fkc.referenced_column_id = referenced_column.column_id
                    INNER JOIN sys.foreign_keys AS fk
                                ON fkc.constraint_object_id = fk.object_id
                                    AND fkc.parent_object_id = fk.parent_object_id
            WHERE parent_table.is_ms_shipped = 0
            AND referenced_table.is_ms_shipped = 0
            ORDER BY table_name, constraint_name, ordinal_position
        "#};

        #[allow(clippy::too_many_arguments)]
        fn get_ids(
            namespace: String,
            table_name: String,
            column_name: &str,
            referenced_schema_name: String,
            referenced_table_name: String,
            referenced_column_name: &str,
            table_ids: &IndexMap<(String, String), TableId>,
            sql_schema: &SqlSchema,
        ) -> Option<(TableId, TableColumnId, TableId, TableColumnId)> {
            let table_id = *table_ids.get(&(namespace, table_name))?;
            let referenced_table_id = *table_ids.get(&(referenced_schema_name, referenced_table_name))?;
            let column_id = sql_schema.walk(table_id).column(column_name)?.id;
            let referenced_column_id = sql_schema.walk(referenced_table_id).column(referenced_column_name)?.id;

            Some((table_id, column_id, referenced_table_id, referenced_column_id))
        }

        let result_set = self.conn.query_raw(sql, &[]).await?;
        let mut current_fk: Option<(String, ForeignKeyId)> = None;

        for row in result_set.into_iter() {
            let namespace = row.get_expect_string("schema_name");

            if !sql_schema.namespaces.contains(&namespace) {
                continue;
            }

            let table_name = row.get_expect_string("table_name");
            let constraint_name = row.get_expect_string("constraint_name");
            let column = row.get_expect_string("column_name");
            let referenced_schema_name = row.get_expect_string("referenced_schema_name");
            let referenced_column = row.get_expect_string("referenced_column_name");
            let referenced_table = row.get_expect_string("referenced_table_name");

            if !sql_schema.namespaces.contains(&referenced_schema_name) {
                return Err(DescriberError::from(DescriberErrorKind::CrossSchemaReference {
                    from: format!("{namespace}.{table_name}"),
                    to: format!("{referenced_schema_name}.{referenced_table}"),
                    constraint: constraint_name,
                    missing_namespace: referenced_schema_name,
                }));
            }

            let (table_id, column_id, referenced_table_id, referenced_column_id) = if let Some(ids) = get_ids(
                namespace,
                table_name,
                &column,
                referenced_schema_name,
                referenced_table,
                &referenced_column,
                table_ids,
                sql_schema,
            ) {
                ids
            } else {
                continue;
            };

            let on_delete_action = match row.get_expect_i64("delete_referential_action") {
                0 => ForeignKeyAction::NoAction,
                1 => ForeignKeyAction::Cascade,
                2 => ForeignKeyAction::SetNull,
                3 => ForeignKeyAction::SetDefault,
                s => panic!("Unrecognized on delete action '{s}'"),
            };

            let on_update_action = match row.get_expect_i64("update_referential_action") {
                0 => ForeignKeyAction::NoAction,
                1 => ForeignKeyAction::Cascade,
                2 => ForeignKeyAction::SetNull,
                3 => ForeignKeyAction::SetDefault,
                s => panic!("Unrecognized on delete action '{s}'"),
            };

            match &current_fk {
                Some((current_constraint_name, _)) if *current_constraint_name == constraint_name => (),
                None | Some(_) => {
                    let fkid = sql_schema.push_foreign_key(
                        Some(constraint_name.clone()),
                        [table_id, referenced_table_id],
                        [on_delete_action, on_update_action],
                    );

                    current_fk = Some((constraint_name, fkid));
                }
            }

            if let Some((_, fkid)) = current_fk {
                sql_schema.push_foreign_key_column(fkid, [column_id, referenced_column_id]);
            }
        }

        Ok(())
    }

    fn get_column_type(
        &self,
        data_type: &str,
        character_maximum_length: Option<i64>,
        numeric_precision: Option<u32>,
        numeric_scale: Option<u32>,
        arity: ColumnArity,
    ) -> ColumnType {
        use ColumnTypeFamily::*;

        // TODO: can we achieve this more elegantly?
        let params = match data_type {
            "numeric" | "decimal" => match (numeric_precision, numeric_scale) {
                (Some(p), Some(s)) => Cow::from(format!("({p},{s})")),
                (None, None) => Cow::from(""),
                _ => unreachable!("Unexpected params for a decimal field."),
            },
            "float" => match numeric_precision {
                Some(p) => Cow::from(format!("({p})")),
                None => Cow::from(""),
            },
            "varchar" | "nvarchar" | "varbinary" => match character_maximum_length {
                Some(-1) => Cow::from("(max)"),
                Some(length) => Cow::from(format!("({length})")),
                None => Cow::from(""),
            },
            "char" | "nchar" | "binary" => match character_maximum_length {
                Some(-1) => unreachable!("Cannot have a `max` variant for type `{}`", data_type),
                Some(length) => Cow::from(format!("({length})")),
                None => Cow::from(""),
            },
            _ => Cow::from(""),
        };

        let full_data_type = format!("{data_type}{params}");

        let casted_character_maximum_length = character_maximum_length.map(|x| x as u32);
        let type_parameter = parse_type_parameter(character_maximum_length);
        let unsupported_type = || (Unsupported(full_data_type.clone()), None);

        let (family, native_type) = match data_type {
            "tinyint" => (Int, Some(MsSqlType::TinyInt)),
            "smallint" => (Int, Some(MsSqlType::SmallInt)),
            "int" => (Int, Some(MsSqlType::Int)),
            "bigint" => (BigInt, Some(MsSqlType::BigInt)),
            "numeric" => match (numeric_precision, numeric_scale) {
                (Some(p), Some(s)) => (Decimal, Some(MsSqlType::Decimal(Some((p, s))))),
                (None, None) => (Decimal, Some(MsSqlType::Decimal(Some((18, 0))))),
                _ => unreachable!("Unexpected params for a numeric field."),
            },
            "decimal" => match (numeric_precision, numeric_scale) {
                (Some(p), Some(s)) => (Decimal, Some(MsSqlType::Decimal(Some((p, s))))),
                (None, None) => (Decimal, Some(MsSqlType::Decimal(Some((18, 0))))),
                _ => unreachable!("Unexpected params for a decimal field."),
            },
            "money" => (Float, Some(MsSqlType::Money)),
            "smallmoney" => (Float, Some(MsSqlType::SmallMoney)),
            "bit" => (Boolean, Some(MsSqlType::Bit)),
            "float" => (Float, Some(MsSqlType::Float(numeric_precision))),
            "real" => (Float, Some(MsSqlType::Real)),
            "date" => (DateTime, Some(MsSqlType::Date)),
            "time" => (DateTime, Some(MsSqlType::Time)),
            "datetime" => (DateTime, Some(MsSqlType::DateTime)),
            "datetime2" => (DateTime, Some(MsSqlType::DateTime2)),
            "datetimeoffset" => (DateTime, Some(MsSqlType::DateTimeOffset)),
            "smalldatetime" => (DateTime, Some(MsSqlType::SmallDateTime)),
            "char" => (String, Some(MsSqlType::Char(casted_character_maximum_length))),
            "nchar" => (String, Some(MsSqlType::NChar(casted_character_maximum_length))),
            "varchar" => (String, Some(MsSqlType::VarChar(type_parameter))),
            "text" => (String, Some(MsSqlType::Text)),
            "nvarchar" => (String, Some(MsSqlType::NVarChar(type_parameter))),
            "ntext" => (String, Some(MsSqlType::NText)),
            "binary" => (Binary, Some(MsSqlType::Binary(casted_character_maximum_length))),
            "varbinary" => (Binary, Some(MsSqlType::VarBinary(type_parameter))),
            "image" => (Binary, Some(MsSqlType::Image)),
            "xml" => (String, Some(MsSqlType::Xml)),
            "uniqueidentifier" => (Uuid, Some(MsSqlType::UniqueIdentifier)),
            _ => unsupported_type(),
        };

        ColumnType {
            full_data_type,
            family,
            arity,
            native_type: native_type.map(NativeTypeInstance::new::<MsSqlType>),
        }
    }
}

fn parse_type_parameter(character_maximum_length: Option<i64>) -> Option<MsSqlTypeParameter> {
    match character_maximum_length {
        Some(-1) => Some(MsSqlTypeParameter::Max),
        Some(x) => Some(MsSqlTypeParameter::Number(x as u16)),
        None => None,
    }
}
