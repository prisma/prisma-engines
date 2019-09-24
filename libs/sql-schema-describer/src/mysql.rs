use super::*;
use log::debug;
use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

pub struct SqlSchemaDescriber {
    conn: Arc<dyn SqlConnection>,
}

impl super::SqlSchemaDescriberBackend for SqlSchemaDescriber {
    fn list_databases(&self) -> SqlSchemaDescriberResult<Vec<String>> {
        Ok(vec![])
    }

    fn describe(&self, schema: &str) -> SqlSchemaDescriberResult<SqlSchema> {
        debug!("describing schema '{}'", schema);
        let tables = self
            .get_table_names(schema)
            .into_iter()
            .map(|t| self.get_table(schema, &t))
            .collect();
        Ok(SqlSchema {
            tables,
            enums: vec![],
            sequences: vec![],
        })
    }
}

impl SqlSchemaDescriber {
    /// Constructor.
    pub fn new(conn: Arc<dyn SqlConnection>) -> SqlSchemaDescriber {
        SqlSchemaDescriber { conn }
    }

    fn get_table_names(&self, schema: &str) -> Vec<String> {
        debug!("Getting table names");
        let sql = "SELECT table_name as table_name FROM information_schema.tables
            WHERE table_schema = ?
            -- Views are not supported yet
            AND table_type = 'BASE TABLE'
            ORDER BY table_name";
        let rows = self
            .conn
            .query_raw(sql, schema, &[schema.into()])
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

    fn get_table(&self, schema: &str, name: &str) -> Table {
        debug!("Getting table '{}'", name);
        let columns = self.get_columns(schema, name);
        let foreign_keys = self.get_foreign_keys(schema, name);
        let fk_cols = foreign_keys
            .iter()
            .flat_map(|fk| fk.columns.iter().map(|col| col.clone()))
            .collect();
        let (indices, primary_key) = self.get_indices(schema, name, fk_cols);
        Table {
            name: name.to_string(),
            columns,
            foreign_keys,
            indices,
            primary_key,
        }
    }

    fn get_columns(&self, schema: &str, table: &str) -> Vec<Column> {
        let sql = "SELECT column_name, data_type, column_default, is_nullable, extra
            FROM information_schema.columns
            WHERE table_schema = ? AND table_name = ?
            ORDER BY column_name";
        let rows = self
            .conn
            .query_raw(sql, schema, &[schema.into(), table.into()])
            .expect("querying for columns");
        let cols = rows
            .into_iter()
            .map(|col| {
                debug!("Got column: {:?}", col);
                let data_type = col.get("data_type").and_then(|x| x.to_string()).expect("get data_type");
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
                let tpe = get_column_type(data_type.as_ref());
                let arity = if tpe.raw.starts_with("_") {
                    ColumnArity::List
                } else if is_required {
                    ColumnArity::Required
                } else {
                    ColumnArity::Nullable
                };
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
                    arity,
                    default: col.get("column_default").and_then(|x| x.to_string()),
                    auto_increment: auto_increment,
                }
            })
            .collect();

        debug!("Found table columns: {:?}", cols);
        cols
    }

    fn get_foreign_keys(&self, schema: &str, table: &str) -> Vec<ForeignKey> {
        // XXX: Is constraint_name unique? Need a way to uniquely associate rows with foreign keys
        // One should think it's unique since it's used to join information_schema.key_column_usage
        // and information_schema.referential_constraints tables in this query lifted from
        // Stack Overflow
        let sql = "SELECT kcu.constraint_name, kcu.column_name, kcu.referenced_table_name, 
            kcu.referenced_column_name, kcu.ordinal_position, rc.delete_rule
            FROM information_schema.key_column_usage AS kcu
            INNER JOIN information_schema.referential_constraints AS rc ON
            kcu.constraint_name = rc.constraint_name
            WHERE kcu.table_schema = ? AND kcu.table_name = ? AND 
            referenced_column_name IS NOT NULL
        ";
        debug!("describing table foreign keys, SQL: '{}'", sql);

        let result_set = self
            .conn
            .query_raw(sql, schema, &[schema.into(), table.into()])
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

    fn get_indices(&self, schema: &str, table_name: &str, fk_cols: Vec<String>) -> (Vec<Index>, Option<PrimaryKey>) {
        let sql = "
            SELECT DISTINCT
                index_name, non_unique, column_name, seq_in_index
            FROM INFORMATION_SCHEMA.STATISTICS
            WHERE table_schema = ? AND table_name = ?
            ORDER BY index_name, seq_in_index
            ";
        debug!("describing indices, SQL: {}", sql);
        let rows = self
            .conn
            .query_raw(sql, schema, &[schema.into(), table_name.into()])
            .expect("querying for indices");

        // Multi-column indices will return more than one row (with different column_name values).
        // We cannot assume that one row corresponds to one index.

        let mut primary_key: Option<PrimaryKey> = None;
        let mut indices_map = BTreeMap::new();

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
            } else if fk_cols.contains(&column_name) {
                ()
            } else {
                if indices_map.contains_key(&index_name) {
                    indices_map.get_mut(&index_name).map(|index: &mut Index| {
                        index.columns.push(column_name);
                    });
                } else {
                    indices_map.insert(
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

        let indices = indices_map.into_iter().map(|(_k, v)| v).collect();

        debug!("Found table indices: {:?}, primary key: {:?}", indices, primary_key);
        (indices, primary_key)
    }
}

fn get_column_type(data_type: &str) -> ColumnType {
    let family = match data_type {
        "int" => ColumnTypeFamily::Int,
        "smallint" => ColumnTypeFamily::Int,
        "tinyint" => ColumnTypeFamily::Boolean,
        "mediumint" => ColumnTypeFamily::Int,
        "bigint" => ColumnTypeFamily::Int,
        "decimal" => ColumnTypeFamily::Float,
        "numeric" => ColumnTypeFamily::Float,
        "float" => ColumnTypeFamily::Float,
        "double" => ColumnTypeFamily::Float,
        "date" => ColumnTypeFamily::DateTime,
        "time" => ColumnTypeFamily::DateTime,
        "datetime" => ColumnTypeFamily::DateTime,
        "timestamp" => ColumnTypeFamily::DateTime,
        "year" => ColumnTypeFamily::DateTime,
        "char" => ColumnTypeFamily::String,
        "varchar" => ColumnTypeFamily::String,
        "text" => ColumnTypeFamily::String,
        "tinytext" => ColumnTypeFamily::String,
        "mediumtext" => ColumnTypeFamily::String,
        "longtext" => ColumnTypeFamily::String,
        // XXX: Is this correct?
        "enum" => ColumnTypeFamily::String,
        "set" => ColumnTypeFamily::String,
        "binary" => ColumnTypeFamily::Binary,
        "varbinary" => ColumnTypeFamily::Binary,
        "blob" => ColumnTypeFamily::Binary,
        "tinyblob" => ColumnTypeFamily::Binary,
        "mediumblob" => ColumnTypeFamily::Binary,
        "longblob" => ColumnTypeFamily::Binary,
        "geometry" => ColumnTypeFamily::Geometric,
        "point" => ColumnTypeFamily::Geometric,
        "linestring" => ColumnTypeFamily::Geometric,
        "polygon" => ColumnTypeFamily::Geometric,
        "multipoint" => ColumnTypeFamily::Geometric,
        "multilinestring" => ColumnTypeFamily::Geometric,
        "multipolygon" => ColumnTypeFamily::Geometric,
        "geometrycollection" => ColumnTypeFamily::Geometric,
        "json" => ColumnTypeFamily::Json,
        x => panic!(format!("type '{}' is not supported here yet.", x)),
    };
    ColumnType {
        raw: data_type.to_string(),
        family: family,
    }
}
