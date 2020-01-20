use super::*;
use log::debug;
use once_cell::sync::Lazy;
use quaint::prelude::Queryable;
use std::collections::{BTreeMap, HashMap};
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

        let table_names = self.get_table_names(schema).await;

        let mut tables = Vec::with_capacity(table_names.len());

        for table_name in &table_names {
            tables.push(self.get_table(schema, table_name).await);
        }

        Ok(SqlSchema {
            tables,
            enums: vec![],
            sequences: vec![],
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
        let sql = "select schema_name as schema_name from information_schema.schemata;";
        let rows = self.conn.query_raw(sql, &[]).await.expect("get schema names ");
        let names = rows
            .into_iter()
            .map(|row| {
                row.get("schema_name")
                    .and_then(|x| x.to_string())
                    .expect("convert schema names")
            })
            .collect();

        debug!("Found schema names: {:?}", names);
        names
    }

    async fn get_table_names(&self, schema: &str) -> Vec<String> {
        debug!("Getting table names");
        let sql = "SELECT table_name as table_name FROM information_schema.tables
            WHERE table_schema = ?
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
        let sql = "SELECT 
      SUM(data_length + index_length) as size 
      FROM information_schema.TABLES 
      WHERE table_schema = ?";
        let result = self.conn.query_raw(sql, &[schema.into()]).await.expect("get db size ");
        let size = result
            .first()
            .map(|row| row.get("size").and_then(|x| x.to_string()).unwrap_or("0".to_string()))
            .unwrap();

        debug!("Found db size: {:?}", size);
        size.parse().unwrap()
    }

    async fn get_table(&self, schema: &str, name: &str) -> Table {
        debug!("Getting table '{}'", name);
        let columns = self.get_columns(schema, name).await;
        let foreign_keys = self.get_foreign_keys(schema, name).await;
        let (indices, primary_key) = self.get_indices(schema, name).await;
        Table {
            name: name.to_string(),
            columns,
            foreign_keys,
            indices,
            primary_key,
        }
    }

    async fn get_columns(&self, schema: &str, table: &str) -> Vec<Column> {
        // We alias all the columns because MySQL column names are case-insensitive in queries, but the
        // information schema column names became upper-case in MySQL 8, causing the code fetching
        // the result values by column name below to fail.
        let sql = "
            SELECT column_name column_name, data_type data_type, column_type full_data_type, column_default column_default, is_nullable is_nullable, extra extra
            FROM information_schema.columns
            WHERE table_schema = ? AND table_name = ?
            ORDER BY column_name";

        let rows = self
            .conn
            .query_raw(sql, &[schema.into(), table.into()])
            .await
            .expect("querying for columns");
        let cols = rows
            .into_iter()
            .map(|col| {
                debug!("Got column: {:?}", col);

                let data_type = col.get("data_type").and_then(|x| x.to_string()).expect("get data_type");
                let full_data_type = col
                    .get("full_data_type")
                    .and_then(|x| x.to_string())
                    .expect("get full_data_type aka column_type");
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
                let tpe = get_column_type(&data_type, &full_data_type, arity);
                let extra = col
                    .get("extra")
                    .and_then(|x| x.to_string())
                    .expect("get extra")
                    .to_lowercase();
                let auto_increment = match extra.as_str() {
                    "auto_increment" => true,
                    _ => false,
                };
                Column {
                    name: col
                        .get("column_name")
                        .and_then(|x| x.to_string())
                        .expect("get column name"),
                    tpe,
                    default: col
                        .get("column_default")
                        .and_then(|x| x.as_str())
                        .and_then(sanitize_default_value)
                        .map(String::from),
                    auto_increment: auto_increment,
                }
            })
            .collect();

        debug!("Found table columns: {:?}", cols);
        cols
    }

    async fn get_foreign_keys(&self, schema: &str, table: &str) -> Vec<ForeignKey> {
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
                rc.delete_rule delete_rule
            FROM information_schema.key_column_usage AS kcu
            INNER JOIN information_schema.referential_constraints AS rc ON
            kcu.constraint_name = rc.constraint_name
            WHERE
                kcu.table_schema = ?
                AND kcu.table_name = ?
                AND rc.constraint_schema = ?
                AND referenced_column_name IS NOT NULL
            ORDER BY ordinal_position
        ";

        debug!("describing table foreign keys, SQL: '{}'", sql);

        let result_set = self
            .conn
            .query_raw(sql, &[schema.into(), table.into(), schema.into()])
            .await
            .expect("querying for foreign keys");
        let mut intermediate_fks: HashMap<String, ForeignKey> = HashMap::new();
        for row in result_set.into_iter() {
            debug!("Got description FK row {:#?}", row);
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
                s @ _ => panic!(format!("Unrecognized on delete action '{}'", s)),
            };

            // Foreign keys covering multiple columns will return multiple rows, which we need to
            // merge.
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
                    };
                    intermediate_fks.insert(constraint_name, fk);
                }
            };
        }

        let mut fks: Vec<ForeignKey> = intermediate_fks
            .values()
            .map(|intermediate_fk| intermediate_fk.to_owned())
            .collect();
        for fk in fks.iter() {
            debug!(
                "Found foreign key - column(s): {:?}, to table: '{}', to column(s): {:?}",
                fk.columns, fk.referenced_table, fk.referenced_columns
            );
        }

        fks.sort_unstable_by_key(|fk| fk.columns.clone());

        fks
    }

    async fn get_indices(&self, schema: &str, table_name: &str) -> (Vec<Index>, Option<PrimaryKey>) {
        // We alias all the columns because MySQL column names are case-insensitive in queries, but the
        // information schema column names became upper-case in MySQL 8, causing the code fetching
        // the result values by column name below to fail.
        let sql = "
            SELECT DISTINCT
                index_name AS index_name,
                non_unique AS non_unique,
                column_name AS column_name,
                seq_in_index AS seq_in_index
            FROM INFORMATION_SCHEMA.STATISTICS
            WHERE table_schema = ? AND table_name = ?
            ORDER BY index_name, seq_in_index
            ";
        debug!("describing indices, SQL: {}", sql);
        let rows = self
            .conn
            .query_raw(sql, &[schema.into(), table_name.into()])
            .await
            .expect("querying for indices");

        // Multi-column indices will return more than one row (with different column_name values).
        // We cannot assume that one row corresponds to one index.

        let mut primary_key: Option<PrimaryKey> = None;
        let mut indexes_map: BTreeMap<String, Index> = BTreeMap::new();

        for row in rows {
            debug!("Got index row: {:#?}", row);
            let seq_in_index = row.get("seq_in_index").and_then(|x| x.as_i64()).expect("seq_in_index");
            let pos = seq_in_index - 1;
            let index_name = row.get("index_name").and_then(|x| x.to_string()).expect("index_name");
            let is_unique = !row.get("non_unique").and_then(|x| x.as_bool()).expect("non_unique");
            let column_name = row.get("column_name").and_then(|x| x.to_string()).expect("column_name");
            let is_pk = index_name.to_lowercase() == "primary";
            if is_pk {
                debug!("Column '{}' is part of the primary key", column_name);
                match primary_key.as_mut() {
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
                        primary_key = Some(PrimaryKey {
                            columns: vec![column_name],
                            sequence: None,
                        });
                    }
                };
            } else {
                if indexes_map.contains_key(&index_name) {
                    indexes_map.get_mut(&index_name).map(|index: &mut Index| {
                        index.columns.push(column_name);
                    });
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
        }

        let indices = indexes_map.into_iter().map(|(_k, v)| v).collect();

        debug!("Found table indices: {:?}, primary key: {:?}", indices, primary_key);
        (indices, primary_key)
    }
}

fn get_column_type(data_type: &str, full_data_type: &str, arity: ColumnArity) -> ColumnType {
    let family = match (data_type, full_data_type) {
        ("int", _) => ColumnTypeFamily::Int,
        ("smallint", _) => ColumnTypeFamily::Int,
        ("tinyint", "tinyint(1)") => ColumnTypeFamily::Boolean,
        ("tinyint", _) => ColumnTypeFamily::Int,
        ("mediumint", _) => ColumnTypeFamily::Int,
        ("bigint", _) => ColumnTypeFamily::Int,
        ("decimal", _) => ColumnTypeFamily::Float,
        ("numeric", _) => ColumnTypeFamily::Float,
        ("float", _) => ColumnTypeFamily::Float,
        ("double", _) => ColumnTypeFamily::Float,
        ("date", _) => ColumnTypeFamily::DateTime,
        ("time", _) => ColumnTypeFamily::DateTime,
        ("datetime", _) => ColumnTypeFamily::DateTime,
        ("timestamp", _) => ColumnTypeFamily::DateTime,
        ("year", _) => ColumnTypeFamily::DateTime,
        ("char", _) => ColumnTypeFamily::String,
        ("varchar", _) => ColumnTypeFamily::String,
        ("text", _) => ColumnTypeFamily::String,
        ("tinytext", _) => ColumnTypeFamily::String,
        ("mediumtext", _) => ColumnTypeFamily::String,
        ("longtext", _) => ColumnTypeFamily::String,
        // XXX: Is this correct?
        ("enum", _) => ColumnTypeFamily::String,
        ("set", _) => ColumnTypeFamily::String,
        ("binary", _) => ColumnTypeFamily::Binary,
        ("varbinary", _) => ColumnTypeFamily::Binary,
        ("blob", _) => ColumnTypeFamily::Binary,
        ("tinyblob", _) => ColumnTypeFamily::Binary,
        ("mediumblob", _) => ColumnTypeFamily::Binary,
        ("longblob", _) => ColumnTypeFamily::Binary,
        ("geometry", _) => ColumnTypeFamily::Geometric,
        ("point", _) => ColumnTypeFamily::Geometric,
        ("linestring", _) => ColumnTypeFamily::Geometric,
        ("polygon", _) => ColumnTypeFamily::Geometric,
        ("multipoint", _) => ColumnTypeFamily::Geometric,
        ("multilinestring", _) => ColumnTypeFamily::Geometric,
        ("multipolygon", _) => ColumnTypeFamily::Geometric,
        ("geometrycollection", _) => ColumnTypeFamily::Geometric,
        ("json", _) => ColumnTypeFamily::Json,
        _ => ColumnTypeFamily::Unknown,
    };
    ColumnType {
        raw: data_type.to_string(),
        family: family,
        arity,
    }
}

fn sanitize_default_value(value: &str) -> Option<&str> {
    match value {
        "NULL" => None,
        default if default.starts_with("'") => Some(unquote_mariadb_string_defaults(default)),
        other => Some(other),
    }
}

fn unquote_mariadb_string_defaults(default: &str) -> &str {
    /// Regex for matching the quotes on the introspected string values on MariaDB.
    static MARIADB_STRING_DEFAULT_RE: Lazy<regex::Regex> = Lazy::new(|| regex::Regex::new(r#"^'(.*)'$"#).unwrap());

    MARIADB_STRING_DEFAULT_RE
        .captures(default)
        .and_then(|captures| captures.get(1))
        .map(|capt| capt.as_str())
        .unwrap_or(default)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mariadb_string_default_regex_works() {
        let quoted_str = "'abc $$ def'";

        assert_eq!(unquote_mariadb_string_defaults(quoted_str), "abc $$ def");

        assert_eq!(unquote_mariadb_string_defaults("heh "), "heh ");
    }
}
