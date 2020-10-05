use super::*;
use once_cell::sync::Lazy;
use quaint::{prelude::Queryable, single::Quaint};
use regex::Regex;
use std::{
    collections::{BTreeMap, HashMap, HashSet},
    convert::TryInto,
};

static DEFAULT_INT: Lazy<Regex> = Lazy::new(|| Regex::new(r"\(\((.*)\)\)").unwrap());
static DEFAULT_NON_INT: Lazy<Regex> = Lazy::new(|| Regex::new(r"\('(.*)'\)").unwrap());
static DEFAULT_DB_GEN: Lazy<Regex> = Lazy::new(|| Regex::new(r"\((.*)\)").unwrap());

pub struct SqlSchemaDescriber {
    conn: Quaint,
}

#[async_trait::async_trait]
impl super::SqlSchemaDescriberBackend for SqlSchemaDescriber {
    async fn list_databases(&self) -> crate::SqlSchemaDescriberResult<Vec<String>> {
        Ok(self.get_databases().await)
    }

    async fn get_metadata(&self, schema: &str) -> crate::SqlSchemaDescriberResult<SQLMetadata> {
        let table_count = self.get_table_names(schema).await.len();
        let size_in_bytes = self.get_size(schema).await;

        Ok(SQLMetadata {
            table_count,
            size_in_bytes,
        })
    }

    async fn describe(&self, schema: &str) -> crate::SqlSchemaDescriberResult<crate::SqlSchema> {
        debug!("describing schema '{}'", schema);

        let mut columns = self.get_all_columns(schema).await;
        let mut indexes = self.get_all_indices(schema).await;
        let mut foreign_keys = self.get_foreign_keys(schema).await;

        let table_names = self.get_table_names(schema).await;
        let mut tables = Vec::with_capacity(table_names.len());

        for table_name in table_names {
            let table = self.get_table(&table_name, &mut columns, &mut indexes, &mut foreign_keys);
            tables.push(table);
        }

        Ok(SqlSchema {
            tables,
            enums: vec![],
            sequences: vec![],
        })
    }

    async fn version(&self, schema: &str) -> crate::SqlSchemaDescriberResult<Option<String>> {
        debug!("getting db version '{}'", schema);
        Ok(self.conn.version().await.unwrap())
    }
}

impl SqlSchemaDescriber {
    pub fn new(conn: Quaint) -> Self {
        Self { conn }
    }

    async fn get_databases(&self) -> Vec<String> {
        debug!("Getting databases");

        let sql = "SELECT name FROM sys.schemas";
        let rows = self.conn.query_raw(sql, &[]).await.expect("get schema names");

        let names = rows
            .into_iter()
            .map(|row| {
                row.get("name")
                    .and_then(|x| x.to_string())
                    .expect("convert schema names")
            })
            .collect();

        debug!("Found schema names: {:?}", names);

        names
    }

    async fn get_table_names(&self, schema: &str) -> Vec<String> {
        debug!("Getting table names");

        let select = r#"
            SELECT table_name
            FROM information_schema.tables t
            INNER JOIN sys.tables st
                ON TABLE_NAME = st.name
                AND SCHEMA_ID(t.TABLE_SCHEMA) = st.schema_id
            WHERE table_schema = @P1
            AND st.is_ms_shipped = 0
            AND table_type = 'BASE TABLE'
            ORDER BY table_name ASC
        "#;

        let rows = self
            .conn
            .query_raw(select, &[schema.into()])
            .await
            .expect("get table names");

        let names = rows
            .into_iter()
            .map(|row| {
                row.get("table_name")
                    .and_then(|x| x.to_string())
                    .expect("get table name")
            })
            .collect();

        debug!("Found table names: {:?}", names);

        names
    }

    async fn get_size(&self, schema: &str) -> usize {
        debug!("Getting db size");

        let sql = r#"
            SELECT
                SUM(a.total_pages) * 8000 AS size
            FROM
                sys.tables t
            INNER JOIN
                sys.partitions p ON t.object_id = p.object_id
            INNER JOIN
                sys.allocation_units a ON p.partition_id = a.container_id
            WHERE schema_name(t.schema_id) = @P1
                AND t.is_ms_shipped = 0
            GROUP BY
                t.schema_id
            ORDER BY
                size DESC;
        "#;

        let rows = self.conn.query_raw(sql, &[schema.into()]).await.expect("get db size");

        let size: i64 = rows
            .into_single()
            .map(|row| row.get("size").and_then(|x| x.as_i64()).unwrap_or(0))
            .unwrap_or(0);

        size.try_into().unwrap()
    }

    fn get_table(
        &self,
        name: &str,
        columns: &mut HashMap<String, Vec<Column>>,
        indexes: &mut HashMap<String, (BTreeMap<String, Index>, Option<PrimaryKey>)>,
        foreign_keys: &mut HashMap<String, Vec<ForeignKey>>,
    ) -> Table {
        let columns = columns.remove(name).expect("table columns not found");
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

    async fn get_all_columns(&self, schema: &str) -> HashMap<String, Vec<Column>> {
        let sql = r#"
            SELECT
                column_name,
                data_type,
                character_maximum_length,
                column_default,
                is_nullable,
                columnproperty(object_id(@P1 + '.' + table_name), column_name, 'IsIdentity') is_identity,
                table_name
            FROM information_schema.columns c
            INNER JOIN sys.tables t
            ON c.TABLE_NAME = t.name AND SCHEMA_ID(c.TABLE_SCHEMA) = t.schema_id
            WHERE table_schema = @P1
            AND t.is_ms_shipped = 'false'
            ORDER BY ordinal_position
        "#;

        let mut map = HashMap::new();

        let rows = self
            .conn
            .query_raw(sql, &[schema.into()])
            .await
            .expect("querying for columns");

        for col in rows {
            debug!("Got column: {:?}", col);

            let table_name = col
                .get("table_name")
                .and_then(|x| x.to_string())
                .expect("get table name");

            let name = col
                .get("column_name")
                .and_then(|x| x.to_string())
                .expect("get column name");

            let data_type = col.get("data_type").and_then(|x| x.to_string()).expect("get data_type");

            let character_maximum_length = col
                .get("character_maximum_length")
                .and_then(|x| x.as_i64().map(|x| x as u32));

            let is_nullable = col
                .get("is_nullable")
                .and_then(|x| x.to_string())
                .expect("get is_nullable")
                .to_lowercase();

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

            let tpe = self.get_column_type(&data_type, character_maximum_length, arity);

            let auto_increment = col
                .get("is_identity")
                .and_then(|x| x.as_bool())
                .expect("get is_identity");

            let entry = map.entry(table_name).or_insert_with(Vec::new);

            let default = match col.get("column_default") {
                None => None,
                Some(param_value) => match param_value.to_string() {
                    None => None,
                    Some(x) if x == "(NULL)" => None,
                    Some(default_string) => {
                        let default_string = DEFAULT_INT
                            .captures_iter(&default_string)
                            .next()
                            .or_else(|| DEFAULT_NON_INT.captures_iter(&default_string).next())
                            .or_else(|| DEFAULT_DB_GEN.captures_iter(&default_string).next())
                            .map(|cap| cap[1].to_string())
                            .expect("Couldn't parse default value");

                        Some(match &tpe.family {
                            ColumnTypeFamily::Int => match parse_int(&default_string) {
                                Some(int_value) => DefaultValue::VALUE(int_value),
                                None => DefaultValue::DBGENERATED(default_string),
                            },
                            ColumnTypeFamily::Float => match parse_float(&default_string) {
                                Some(float_value) => DefaultValue::VALUE(float_value),
                                None => DefaultValue::DBGENERATED(default_string),
                            },
                            ColumnTypeFamily::Decimal => match parse_float(&default_string) {
                                Some(float_value) => DefaultValue::VALUE(float_value),
                                None => DefaultValue::DBGENERATED(default_string),
                            },
                            ColumnTypeFamily::Boolean => match parse_int(&default_string) {
                                Some(PrismaValue::Int(1)) => DefaultValue::VALUE(PrismaValue::Boolean(true)),
                                Some(PrismaValue::Int(0)) => DefaultValue::VALUE(PrismaValue::Boolean(false)),
                                _ => DefaultValue::DBGENERATED(default_string),
                            },
                            ColumnTypeFamily::String => DefaultValue::VALUE(PrismaValue::String(default_string)),
                            //todo check other now() definitions
                            ColumnTypeFamily::DateTime => match default_string.as_str() {
                                "getdate()" => DefaultValue::NOW,
                                _ => DefaultValue::DBGENERATED(default_string),
                            },
                            ColumnTypeFamily::Binary => DefaultValue::DBGENERATED(default_string),
                            ColumnTypeFamily::Json => DefaultValue::DBGENERATED(default_string),
                            ColumnTypeFamily::Uuid => DefaultValue::DBGENERATED(default_string),
                            ColumnTypeFamily::Geometric => DefaultValue::DBGENERATED(default_string),
                            ColumnTypeFamily::LogSequenceNumber => DefaultValue::DBGENERATED(default_string),
                            ColumnTypeFamily::TextSearch => DefaultValue::DBGENERATED(default_string),
                            ColumnTypeFamily::TransactionId => DefaultValue::DBGENERATED(default_string),
                            ColumnTypeFamily::Enum(_) => unreachable!("No enums in MSSQL"),
                            ColumnTypeFamily::Duration => DefaultValue::DBGENERATED(default_string),
                            ColumnTypeFamily::Unsupported(_) => DefaultValue::DBGENERATED(default_string),
                        })
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

        map
    }

    async fn get_all_indices(&self, schema: &str) -> HashMap<String, (BTreeMap<String, Index>, Option<PrimaryKey>)> {
        let mut map = HashMap::new();
        let mut indexes_with_expressions: HashSet<(String, String)> = HashSet::new();

        let sql = r#"
            SELECT DISTINCT
                ind.name AS index_name,
                ind.is_unique AS is_unique,
                ind.is_primary_key AS is_primary_key,
                col.name AS column_name,
                ic.index_column_id AS seq_in_index,
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
            ORDER BY index_name, seq_in_index
        "#;

        let rows = self
            .conn
            .query_raw(sql, &[schema.into()])
            .await
            .expect("querying for indices");

        for row in rows {
            debug!("Got index row: {:#?}", row);

            let table_name = row.get("table_name").and_then(|x| x.to_string()).expect("table_name");
            let index_name = row.get("index_name").and_then(|x| x.to_string()).expect("index_name");

            match row.get("column_name").and_then(|x| x.to_string()) {
                Some(column_name) => {
                    let seq_in_index = row.get("seq_in_index").and_then(|x| x.as_i64()).expect("seq_in_index");
                    let pos = seq_in_index - 1;
                    let is_unique = row.get("is_unique").and_then(|x| x.as_bool()).expect("is_unique");

                    // Multi-column indices will return more than one row (with different column_name values).
                    // We cannot assume that one row corresponds to one index.
                    let (ref mut indexes_map, ref mut primary_key): &mut (_, Option<PrimaryKey>) = map
                        .entry(table_name)
                        .or_insert((BTreeMap::<String, Index>::new(), None));

                    let is_pk = row
                        .get("is_primary_key")
                        .and_then(|x| x.as_bool())
                        .expect("is_primary_key");

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
                                    constraint_name: None,
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

        map
    }

    async fn get_foreign_keys(&self, schema: &str) -> HashMap<String, Vec<ForeignKey>> {
        // Foreign keys covering multiple columns will return multiple rows, which we need to
        // merge.
        let mut map: HashMap<String, HashMap<String, ForeignKey>> = HashMap::new();

        let sql = r#"
            SELECT
                OBJECT_NAME(fk.constraint_object_id) AS constraint_name,
                parent_table.name AS table_name,
                referenced_table.name AS referenced_table_name,
                parent_column.name AS column_name,
                referenced_column.name AS referenced_column_name,
                rc.delete_rule AS delete_rule,
                rc.update_rule AS update_rule,
                kcu.ordinal_position AS ordinal_position
            FROM
                sys.foreign_key_columns AS fk
            INNER JOIN sys.tables AS parent_table
                ON fk.parent_object_id = parent_table.object_id
            INNER JOIN sys.tables AS referenced_table
                ON fk.referenced_object_id = referenced_table.object_id
            INNER JOIN sys.columns AS parent_column
                ON fk.parent_object_id = parent_column.object_id
                AND fk.parent_column_id = parent_column.column_id
            INNER JOIN sys.columns AS referenced_column
                ON fk.referenced_object_id = referenced_column.object_id
                AND fk.referenced_column_id = referenced_column.column_id
            INNER JOIN information_schema.referential_constraints AS rc
                ON OBJECT_NAME(fk.constraint_object_id) = rc.constraint_name
                AND rc.constraint_schema = @P1
            INNER JOIN information_schema.key_column_usage AS kcu
                ON OBJECT_NAME(fk.constraint_object_id) = kcu.constraint_name
                AND parent_column.name = kcu.column_name
                AND parent_table.name = kcu.table_name
                AND kcu.table_schema = @P1
                AND referenced_column.name IS NOT NULL
            WHERE parent_table.is_ms_shipped = 'false'
            AND referenced_table.is_ms_shipped = 'false'
            ORDER BY
                ordinal_position
        "#;

        let result_set = self
            .conn
            .query_raw(sql, &[schema.into()])
            .await
            .expect("querying for foreign keys");

        for row in result_set.into_iter() {
            debug!("Got description FK row {:#?}", row);

            let table_name = row
                .get("table_name")
                .and_then(|x| x.to_string())
                .expect("get table_name");

            let constraint_name = row
                .get("constraint_name")
                .and_then(|x| x.to_string())
                .expect("get constraint_name");

            let column = row
                .get("column_name")
                .and_then(|x| x.to_string())
                .expect("get column_name");

            let referenced_table = row
                .get("referenced_table_name")
                .and_then(|x| x.to_string())
                .expect("get referenced_table_name");

            let referenced_column = row
                .get("referenced_column_name")
                .and_then(|x| x.to_string())
                .expect("get referenced_column_name");

            let ord_pos = row
                .get("ordinal_position")
                .and_then(|x| x.as_i64())
                .expect("get ordinal_position");

            let on_delete_action = match row
                .get("delete_rule")
                .and_then(|x| x.to_string())
                .expect("get delete_rule")
                .to_lowercase()
                .as_str()
            {
                "cascade" => ForeignKeyAction::Cascade,
                "set null" => ForeignKeyAction::SetNull,
                "set default" => ForeignKeyAction::SetDefault,
                "restrict" => ForeignKeyAction::Restrict,
                "no action" => ForeignKeyAction::NoAction,
                s => panic!(format!("Unrecognized on delete action '{}'", s)),
            };

            let on_update_action = match row
                .get("update_rule")
                .and_then(|x| x.to_string())
                .expect("get update_rule")
                .to_lowercase()
                .as_str()
            {
                "cascade" => ForeignKeyAction::Cascade,
                "set null" => ForeignKeyAction::SetNull,
                "set default" => ForeignKeyAction::SetDefault,
                "restrict" => ForeignKeyAction::Restrict,
                "no action" => ForeignKeyAction::NoAction,
                s => panic!(format!("Unrecognized on delete action '{}'", s)),
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

        map.into_iter()
            .map(|(k, v)| {
                let mut fks: Vec<ForeignKey> = v.into_iter().map(|(_k, v)| v).collect();

                fks.sort_unstable_by(|this, other| this.columns.cmp(&other.columns));

                (k, fks)
            })
            .collect()
    }

    fn get_column_type(
        &self,
        data_type: &str,
        character_maximum_length: Option<u32>,
        arity: ColumnArity,
    ) -> ColumnType {
        use ColumnTypeFamily::*;

        let family = match data_type {
            "date" | "time" | "datetime" | "datetime2" | "smalldatetime" | "datetimeoffset" => DateTime,
            "numeric" | "decimal" | "float" | "real" | "smallmoney" | "money" => Float,
            "char" | "nchar" | "varchar" | "nvarchar" | "text" | "ntext" => String,
            "tinyint" | "smallint" | "int" | "bigint" => Int,
            "binary" | "varbinary" | "image" => Binary,
            "uniqueidentifier" => Uuid,
            "bit" => Boolean,
            r#type => Unsupported(r#type.into()),
        };

        ColumnType {
            data_type: data_type.into(),
            full_data_type: data_type.into(),
            character_maximum_length,
            family,
            arity,
            native_type: Default::default(),
        }
    }
}
