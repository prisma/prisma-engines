use crate::{
    getters::Getter, parsers::Parser, Column, ColumnArity, ColumnType, ColumnTypeFamily, DefaultValue, DescriberError,
    DescriberErrorKind, DescriberResult, ForeignKey, ForeignKeyAction, Index, IndexType, PrimaryKey, Procedure,
    SqlMetadata, SqlSchema, Table, UserDefinedType, View,
};
use indoc::indoc;
use native_types::{MsSqlType, MsSqlTypeParameter, NativeType};
use once_cell::sync::Lazy;
use prisma_value::PrismaValue;
use quaint::prelude::Queryable;
use regex::Regex;
use std::{
    any::type_name,
    borrow::Cow,
    collections::{BTreeMap, HashSet},
    convert::TryInto,
};
use tracing::{debug, trace};

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
static DEFAULT_SHARED_CONSTRAINT: Lazy<Regex> = Lazy::new(|| Regex::new(r"^CREATE DEFAULT (.*)").unwrap());

pub struct SqlSchemaDescriber<'a> {
    conn: &'a dyn Queryable,
}

impl std::fmt::Debug for SqlSchemaDescriber<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct(type_name::<SqlSchemaDescriber>()).finish()
    }
}

#[async_trait::async_trait]
impl super::SqlSchemaDescriberBackend for SqlSchemaDescriber<'_> {
    async fn list_databases(&self) -> DescriberResult<Vec<String>> {
        Ok(self.get_databases().await?)
    }

    async fn get_metadata(&self, schema: &str) -> DescriberResult<SqlMetadata> {
        let table_count = self.get_table_names(schema).await?.len();
        let size_in_bytes = self.get_size(schema).await?;

        Ok(SqlMetadata {
            table_count,
            size_in_bytes,
        })
    }

    #[tracing::instrument]
    async fn describe(&self, schema: &str) -> DescriberResult<SqlSchema> {
        let mut columns = self.get_all_columns(schema).await?;
        let mut indexes = self.get_all_indices(schema).await?;
        let mut foreign_keys = self.get_foreign_keys(schema).await?;

        let table_names = self.get_table_names(schema).await?;
        let mut tables = Vec::with_capacity(table_names.len());

        for table_name in table_names {
            let table = self.get_table(&table_name, &mut columns, &mut indexes, &mut foreign_keys);
            tables.push(table);
        }

        let views = self.get_views(schema).await?;
        let procedures = self.get_procedures(schema).await?;
        let user_defined_types = self.get_user_defined_types(schema).await?;

        Ok(SqlSchema {
            tables,
            views,
            procedures,
            enums: vec![],
            sequences: vec![],
            user_defined_types,
        })
    }

    #[tracing::instrument]
    async fn version(&self, _schema: &str) -> DescriberResult<Option<String>> {
        Ok(self.conn.version().await?)
    }
}

impl Parser for SqlSchemaDescriber<'_> {}

impl<'a> SqlSchemaDescriber<'a> {
    pub fn new(conn: &'a dyn Queryable) -> Self {
        Self { conn }
    }

    #[tracing::instrument]
    async fn get_databases(&self) -> DescriberResult<Vec<String>> {
        let sql = "SELECT name FROM sys.schemas";
        let rows = self.conn.query_raw(sql, &[]).await?;

        let names = rows.into_iter().map(|row| row.get_expect_string("name")).collect();

        trace!("Found schema names: {:?}", names);

        Ok(names)
    }

    #[tracing::instrument]
    async fn get_procedures(&self, schema: &str) -> DescriberResult<Vec<Procedure>> {
        let sql = r#"
            SELECT name, OBJECT_DEFINITION(object_id) AS definition
            FROM sys.objects
            WHERE SCHEMA_NAME(schema_id) = @P1
                AND is_ms_shipped = 0
                AND type = 'P';
        "#;

        let rows = self.conn.query_raw(sql, &[schema.into()]).await?;
        let mut procedures = Vec::with_capacity(rows.len());

        for row in rows.into_iter() {
            procedures.push(Procedure {
                name: row.get_expect_string("name"),
                definition: row.get_string("definition"),
            });
        }

        Ok(procedures)
    }

    #[tracing::instrument]
    async fn get_table_names(&self, schema: &str) -> DescriberResult<Vec<String>> {
        let select = r#"
            SELECT t.name AS table_name
            FROM sys.tables t
            WHERE SCHEMA_NAME(t.schema_id) = @P1
            AND t.is_ms_shipped = 0
            AND t.type = 'U'
            ORDER BY t.name asc;
        "#;

        let rows = self.conn.query_raw(select, &[schema.into()]).await?;

        let names = rows
            .into_iter()
            .map(|row| row.get_expect_string("table_name"))
            .collect();

        trace!("Found table names: {:?}", names);

        Ok(names)
    }

    #[tracing::instrument]
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
            .map(|row| row.get("size").and_then(|x| x.as_i64()).unwrap_or(0))
            .unwrap_or(0);

        Ok(size
            .try_into()
            .expect("Invariant violation: size is not a valid usize value."))
    }

    fn get_table(
        &self,
        name: &str,
        columns: &mut BTreeMap<String, Vec<Column>>,
        indexes: &mut BTreeMap<String, (BTreeMap<String, Index>, Option<PrimaryKey>)>,
        foreign_keys: &mut BTreeMap<String, Vec<ForeignKey>>,
    ) -> Table {
        let columns = columns.remove(name).unwrap_or_default();
        let (indices, primary_key) = indexes.remove(name).unwrap_or_else(|| (BTreeMap::new(), None));

        let foreign_keys = foreign_keys.remove(name).unwrap_or_default();

        Table {
            name: name.to_string(),
            columns,
            foreign_keys,
            indices: indices.into_iter().map(|(_k, v)| v).collect(),
            primary_key,
        }
    }

    async fn get_all_columns(&self, schema: &str) -> DescriberResult<BTreeMap<String, Vec<Column>>> {
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
                    ELSE ODBCSCALE(c.system_type_id, c.scale) END) AS numeric_scale
            FROM sys.columns c
                    INNER JOIN sys.tables t ON c.object_id = t.object_id
                    INNER JOIN sys.types typ ON c.user_type_id = typ.user_type_id
            WHERE OBJECT_SCHEMA_NAME(c.object_id) = @P1
            AND t.is_ms_shipped = 0
            ORDER BY COLUMNPROPERTY(c.object_id, c.name, 'ordinal');
        "#};

        let mut map = BTreeMap::new();
        let rows = self.conn.query_raw(sql, &[schema.into()]).await?;

        for col in rows {
            debug!("Got column: {:?}", col);

            let table_name = col.get_expect_string("table_name");

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
            let entry = map.entry(table_name).or_insert_with(Vec::new);

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
                            .ok_or_else(|| format!("Couldn't parse default value: `{}`", default_string))
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

            entry.push(Column {
                name,
                tpe,
                default,
                auto_increment,
            });
        }

        Ok(map)
    }

    async fn get_all_indices(
        &self,
        schema: &str,
    ) -> DescriberResult<BTreeMap<String, (BTreeMap<String, Index>, Option<PrimaryKey>)>> {
        let mut map = BTreeMap::new();
        let mut indexes_with_expressions: HashSet<(String, String)> = HashSet::new();

        let sql = indoc! {r#"
            SELECT DISTINCT
                ind.name AS index_name,
                ind.is_unique AS is_unique,
                ind.is_primary_key AS is_primary_key,
                col.name AS column_name,
                ic.key_ordinal AS seq_in_index,
                t.name AS table_name
            FROM
                sys.indexes ind
            INNER JOIN sys.index_columns ic
                ON ind.object_id = ic.object_id AND ind.index_id = ic.index_id
            INNER JOIN sys.columns col
                ON ic.object_id = col.object_id AND ic.column_id = col.column_id
            INNER JOIN
                sys.tables t ON ind.object_id = t.object_id
            WHERE SCHEMA_NAME(t.schema_id) = @P1
                AND t.is_ms_shipped = 0
                AND ind.filter_definition IS NULL
                AND ind.name IS NOT NULL
                AND ind.type_desc IN (
                    'CLUSTERED',
                    'NONCLUSTERED',
                    'CLUSTERED COLUMNSTORE',
                    'NONCLUSTERED COLUMNSTORE'
                )
            ORDER BY index_name, seq_in_index
        "#};

        let rows = self.conn.query_raw(sql, &[schema.into()]).await?;

        for row in rows {
            trace!("Got index row: {:#?}", row);

            let table_name = row.get_expect_string("table_name");
            let index_name = row.get_expect_string("index_name");

            match row.get("column_name").and_then(|x| x.to_string()) {
                Some(column_name) => {
                    let seq_in_index = row.get_expect_i64("seq_in_index");
                    let pos = seq_in_index - 1;
                    let is_unique = row.get_expect_bool("is_unique");

                    // Multi-column indices will return more than one row (with different column_name values).
                    // We cannot assume that one row corresponds to one index.
                    let (ref mut indexes_map, ref mut primary_key): &mut (_, Option<PrimaryKey>) = map
                        .entry(table_name)
                        .or_insert((BTreeMap::<String, Index>::new(), None));

                    let is_pk = row.get_expect_bool("is_primary_key");

                    if is_pk {
                        debug!("Column '{}' is part of the primary key", column_name);

                        match primary_key {
                            Some(pk) => {
                                if pk.columns.len() < (pos + 1) as usize {
                                    pk.columns.resize((pos + 1) as usize, "".to_string());
                                }

                                pk.columns[pos as usize] = column_name;

                                debug!(
                                    "The primary key has already been created, added column to it: {:?}",
                                    pk.columns
                                );
                            }
                            None => {
                                debug!("Instantiating primary key");

                                primary_key.replace(PrimaryKey {
                                    columns: vec![column_name],
                                    sequence: None,
                                    constraint_name: Some(index_name),
                                });
                            }
                        };
                    } else if indexes_map.contains_key(&index_name) {
                        if let Some(index) = indexes_map.get_mut(&index_name) {
                            index.columns.push(column_name);
                        }
                    } else {
                        indexes_map.insert(
                            index_name.clone(),
                            Index {
                                name: index_name,
                                columns: vec![column_name],
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
        }

        Ok(map)
    }

    #[tracing::instrument]
    async fn get_views(&self, schema: &str) -> DescriberResult<Vec<View>> {
        let sql = indoc! {r#"
            SELECT name AS view_name, OBJECT_DEFINITION(object_id) AS view_sql
            FROM sys.views
            WHERE is_ms_shipped = 0
            AND SCHEMA_NAME(schema_id) = @P1
        "#};

        let result_set = self.conn.query_raw(sql, &[schema.into()]).await?;
        let mut views = Vec::with_capacity(result_set.len());

        for row in result_set.into_iter() {
            views.push(View {
                name: row.get_expect_string("view_name"),
                definition: row.get_string("view_sql"),
            })
        }

        Ok(views)
    }

    async fn get_user_defined_types(&self, schema: &str) -> DescriberResult<Vec<UserDefinedType>> {
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
                            END) AS scale
            FROM sys.types udt
                    LEFT JOIN sys.types systyp
                            ON udt.system_type_id = systyp.user_type_id
            WHERE SCHEMA_NAME(udt.schema_id) = @P1
            AND udt.is_user_defined = 1
        "#};

        let result_set = self.conn.query_raw(sql, &[schema.into()]).await?;

        let types: Vec<UserDefinedType> = result_set
            .into_iter()
            .map(|row| {
                let name = row.get_expect_string("user_type_name");
                let max_length = row.get_i64("max_length");
                let precision = row.get_u32("precision");
                let scale = row.get_u32("scale");

                let definition = row
                    .get_string("system_type_name")
                    .map(|name| match (max_length, precision, scale) {
                        (Some(len), _, _) if len == -1 => format!("{}(max)", name),
                        (Some(len), _, _) => format!("{}({})", name, len),
                        (_, Some(p), Some(s)) => format!("{}({},{})", name, p, s),
                        _ => name,
                    });

                UserDefinedType { name, definition }
            })
            .collect();

        Ok(types)
    }

    async fn get_foreign_keys(&self, schema: &str) -> DescriberResult<BTreeMap<String, Vec<ForeignKey>>> {
        // Foreign keys covering multiple columns will return multiple rows, which we need to
        // merge.
        let mut map: BTreeMap<String, BTreeMap<String, ForeignKey>> = BTreeMap::new();

        let sql = indoc! {r#"
            SELECT OBJECT_NAME(fkc.constraint_object_id) AS constraint_name,
                parent_table.name                       AS table_name,
                referenced_table.name                   AS referenced_table_name,
                SCHEMA_NAME(referenced_table.schema_id) AS referenced_schema_name,
                parent_column.name                      AS column_name,
                referenced_column.name                  AS referenced_column_name,
                fk.delete_referential_action            AS delete_referential_action,
                fk.update_referential_action            AS update_referential_action,
                fkc.constraint_column_id                AS ordinal_position
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
            AND OBJECT_SCHEMA_NAME(fkc.parent_object_id) = @P1
            ORDER BY ordinal_position
        "#};

        let result_set = self.conn.query_raw(sql, &[schema.into()]).await?;

        for row in result_set.into_iter() {
            debug!("Got description FK row {:#?}", row);

            let table_name = row.get_expect_string("table_name");
            let constraint_name = row.get_expect_string("constraint_name");
            let column = row.get_expect_string("column_name");
            let referenced_table = row.get_expect_string("referenced_table_name");
            let referenced_schema_name = row.get_expect_string("referenced_schema_name");
            let referenced_column = row.get_expect_string("referenced_column_name");
            let ord_pos = row.get_expect_i64("ordinal_position");

            if schema != referenced_schema_name {
                return Err(DescriberError::from(DescriberErrorKind::CrossSchemaReference {
                    from: format!("{}.{}", schema, table_name),
                    to: format!("{}.{}", referenced_schema_name, referenced_table),
                    constraint: constraint_name,
                }));
            }

            let on_delete_action = match row.get_expect_i64("delete_referential_action") {
                0 => ForeignKeyAction::NoAction,
                1 => ForeignKeyAction::Cascade,
                2 => ForeignKeyAction::SetNull,
                3 => ForeignKeyAction::SetDefault,
                s => panic!("Unrecognized on delete action '{}'", s),
            };

            let on_update_action = match row.get_expect_i64("update_referential_action") {
                0 => ForeignKeyAction::NoAction,
                1 => ForeignKeyAction::Cascade,
                2 => ForeignKeyAction::SetNull,
                3 => ForeignKeyAction::SetDefault,
                s => panic!("Unrecognized on delete action '{}'", s),
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
                (Some(p), Some(s)) => Cow::from(format!("({},{})", p, s)),
                (None, None) => Cow::from(""),
                _ => unreachable!("Unexpected params for a decimal field."),
            },
            "float" => match numeric_precision {
                Some(p) => Cow::from(format!("({})", p)),
                None => Cow::from(""),
            },
            "varchar" | "nvarchar" | "varbinary" => match character_maximum_length {
                Some(-1) => Cow::from("(max)"),
                Some(length) => Cow::from(format!("({})", length)),
                None => Cow::from(""),
            },
            "char" | "nchar" | "binary" => match character_maximum_length {
                Some(-1) => unreachable!("Cannot have a `max` variant for type `{}`", data_type),
                Some(length) => Cow::from(format!("({})", length)),
                None => Cow::from(""),
            },
            _ => Cow::from(""),
        };

        let full_data_type = format!("{}{}", data_type, params);

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
            native_type: native_type.map(|x| x.to_json()),
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
