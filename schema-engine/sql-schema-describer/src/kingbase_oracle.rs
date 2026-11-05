//! Independent schema description for Kingbase Oracle, using Oracle object
//! dictionaries and explicitly documented KES metadata supplements.
//! Explicit Oracle dictionary queries with KES supplements. C010 ALL_* OWNER
//! denotes the role, not the namespace; some views fold names and omit stable
//! column ordinals. ALL_OBJECTS supplies stable object IDs; sys_class/sys_namespace retain physical
//! names and namespace identity despite dictionary case folding. Columns, index/FK arrays, enums and source decompilation
//! use documented KES interfaces where the Oracle views lose that identity.
//! No PostgreSQL templates, SQL rewriting, or PostgreSQL native-type parser.

mod default;
use crate::{
    Column, ColumnArity, ColumnType, ColumnTypeFamily, DefaultKind, DescriberError, DescriberErrorKind,
    DescriberResult, ForeignKeyAction, IndexColumn, PrismaValue, Procedure, SQLSortOrder, SqlSchema,
    SqlSchemaDescriberBackend, getters::Getter,
};
use psl::{
    builtin_connectors::{KingbaseOracleNumberArguments, KingbaseOracleType},
    datamodel_connector::NativeTypeInstance,
};
use quaint::{
    Value,
    connector::{Queryable, ResultSet},
};
use std::{borrow::Cow, collections::HashMap};

/// Metadata consumed by Kingbase Oracle migration rendering.
#[derive(Default, Debug, Clone)]
pub struct KingbaseOracleSchemaExt {
    /// Constraint identity is independent of the supporting index name.
    pub owning_constraint_names: HashMap<crate::IndexId, String>,
    /// Actual generation mechanism; absent entries never authorize deleting a sequence.
    pub column_generations: HashMap<crate::TableColumnId, ColumnGeneration>,
    /// Server definitions preserve FK properties not represented in PSL.
    pub foreign_key_definitions: HashMap<crate::ForeignKeyId, String>,
}

/// Generation metadata read from documented KES ownership interfaces.
#[derive(Debug, Clone)]
pub enum ColumnGeneration {
    /// A database identity column, removed with DROP IDENTITY.
    Identity {
        /// Identity sequence properties, when exposed.
        sequence: Option<IdentitySequence>,
    },
    /// A referenced sequence owned by this column. Other references prevent cleanup.
    OwnedSequence {
        /// The sequence's physical namespace.
        namespace: String,
        /// The sequence's physical name.
        name: String,
        /// Another object still references this sequence.
        shared: bool,
    },
}

/// Public KES sequence properties needed to preserve shared identity generation.
#[derive(Debug, Clone)]
pub struct IdentitySequence {
    /// Physical namespace.
    pub namespace: String,
    /// Physical name.
    pub name: String,
    /// CREATE SEQUENCE options, derived exclusively from typed catalog values.
    pub options: String,
}

/// Kingbase Oracle folds an ASCII all-uppercase quoted identifier to lowercase,
/// while preserving mixed-case quoted identifiers. Keep the calculated schema
/// consistent with the physical catalog before comparing migration schemas.
pub fn normalize_schema_identifiers(schema: &mut SqlSchema) {
    // Normalization can merge aliases such as APP and app. Remap namespace
    // IDs as well, so later mixed-case namespaces keep their objects.
    let namespace_ids = std::mem::take(&mut schema.namespaces)
        .into_iter()
        .map(|name| schema.push_namespace(normalize_namespace(&name).into_owned()))
        .collect::<Vec<_>>();
    for namespace_id in schema
        .tables
        .iter_mut()
        .map(|table| &mut table.namespace_id)
        .chain(schema.enums.iter_mut().map(|enm| &mut enm.namespace_id))
        .chain(schema.views.iter_mut().map(|view| &mut view.namespace_id))
        .chain(
            schema
                .procedures
                .iter_mut()
                .map(|procedure| &mut procedure.namespace_id),
        )
        .chain(schema.user_defined_types.iter_mut().map(|udt| &mut udt.namespace_id))
    {
        if let Some(id) = namespace_ids.get(namespace_id.0 as usize) {
            *namespace_id = *id;
        }
    }
    if let Some(namespace) = &mut schema.runtime_namespace {
        normalize_identifier(namespace);
    }
    for table in &mut schema.tables {
        normalize_identifier(&mut table.name);
    }
    for (_, column) in &mut schema.table_columns {
        normalize_identifier(&mut column.name);
    }
    for index in &mut schema.indexes {
        normalize_identifier(&mut index.index_name);
    }
    for foreign_key in &mut schema.foreign_keys {
        if let Some(name) = &mut foreign_key.constraint_name {
            normalize_identifier(name);
        }
    }
    for enm in &mut schema.enums {
        normalize_identifier(&mut enm.name);
    }
}

/// Physical spelling of a quoted Oracle-mode schema name. ASCII all-uppercase
/// names fold to lowercase; mixed-case and non-ASCII names remain unchanged.
pub fn normalize_namespace(name: &str) -> Cow<'_, str> {
    if name.is_ascii() && !name.bytes().any(|byte| byte.is_ascii_lowercase()) {
        Cow::Owned(name.to_ascii_lowercase())
    } else {
        Cow::Borrowed(name)
    }
}

/// Keep introspected @@schema annotations consistent with the user's datasource
/// schema list. Migration comparison continues to use physical catalog names.
pub fn preserve_namespace_spelling(schema: &mut SqlSchema, requested: &[String]) {
    schema.namespaces = schema
        .namespaces
        .iter()
        .map(|physical| {
            requested
                .iter()
                .find(|name| normalize_namespace(name).as_ref() == physical)
                .unwrap_or(physical)
                .clone()
        })
        .collect();
}

fn normalize_identifier(name: &mut str) {
    if name.is_ascii() && !name.bytes().any(|byte| byte.is_ascii_lowercase()) {
        name.make_ascii_lowercase();
    }
}

/// Oracle stores the empty character string as NULL. A default of '' is
/// therefore indistinguishable from an absent default in its catalog.
pub fn normalize_empty_string_defaults(schema: &mut SqlSchema) {
    schema.table_default_values.retain(
        |(_, default)| !matches!(default.kind(), DefaultKind::Value(PrismaValue::String(value)) if value.is_empty()),
    );
}

/// Describes Kingbase Oracle without delegating SQL or parsing to PostgreSQL.
pub struct SqlSchemaDescriber<'a> {
    conn: &'a dyn Queryable,
}
impl<'a> SqlSchemaDescriber<'a> {
    /// Use the selected connection's Oracle dictionaries and KES interfaces.
    pub fn new(conn: &'a dyn Queryable) -> Self {
        Self { conn }
    }
}
#[async_trait::async_trait]
impl SqlSchemaDescriberBackend for SqlSchemaDescriber<'_> {
    async fn describe(&self, schemas: &[&str]) -> DescriberResult<SqlSchema> {
        describe_schema(self.conn, schemas).await
    }
    async fn version(&self) -> DescriberResult<Option<String>> {
        Ok(self.conn.version().await?)
    }
}
const OBJECTS: &str = include_str!("kingbase_oracle/tables_query.sql");

// ALL_TAB_COLUMNS cannot disambiguate equal table names in namespaces owned
// by the same role. Resolve the exact relation/ordinal, and read native metadata
// directly; information_schema precision avoids relying on PG type structs.
const COLUMNS: &str = include_str!("kingbase_oracle/columns_query.sql");

const ENUMS: &str = include_str!("kingbase_oracle/enums_query.sql");

// ALL_INDEXES/ALL_IND_COLUMNS fold physical identifiers and lack namespace/OID
// keys. KES exposes exact ownership and ordered index columns by relation OID.
const INDEXES: &str = include_str!("kingbase_oracle/indexes_query.sql");

// ALL_CONSTRAINTS has no ON UPDATE rule in Oracle. Read both KES actions and
// use the same ordinal for the constrained and referenced column arrays.
const FOREIGN_KEYS: &str = include_str!("kingbase_oracle/foreign_keys_query.sql");

const CHECKS: &str = include_str!("kingbase_oracle/constraints_query.sql");

// ALL_PROCEDURES is absent on C010; ALL_SOURCE lacks overload identity and a
// stable namespace key. Use the KES routine decompiler directly, with no PG alias.
const PROCEDURES: &str = include_str!("kingbase_oracle/procedures_query.sql");

async fn identity_sequence(
    conn: &dyn Queryable,
    oid: i64,
    namespace: String,
    name: String,
) -> DescriberResult<IdentitySequence> {
    let properties = conn
        .query_raw(
            include_str!("kingbase_oracle/sequences_query.sql"),
            &[Value::int64(oid)],
        )
        .await?;
    let p = properties.into_iter().next().expect("identity sequence properties");
    let data_type = match p.get_expect_string("typname").as_str() {
        "int2" => "SMALLINT",
        "int4" => "INTEGER",
        _ => "BIGINT",
    };
    let options = format!(
        "AS {data_type} INCREMENT BY {} MINVALUE {} MAXVALUE {} START WITH {} CACHE {} {} CYCLE",
        p.get_expect_i64("seqincrement"),
        p.get_expect_i64("seqmin"),
        p.get_expect_i64("seqmax"),
        p.get_expect_i64("seqstart"),
        p.get_expect_i64("seqcache"),
        if p.get_expect_bool("seqcycle") { "" } else { "NO" }
    );

    Ok(IdentitySequence {
        namespace,
        name,
        options,
    })
}

async fn query(conn: &dyn Queryable, sql: &str, schemas: &[&str]) -> DescriberResult<ResultSet> {
    let placeholders = (1..=schemas.len())
        .map(|i| format!("${i}"))
        .collect::<Vec<_>>()
        .join(",");
    let sql = sql.replace("__SCHEMAS__", &placeholders);
    let result = conn
        .query_raw(&sql, &schemas.iter().map(|s| Value::text(*s)).collect::<Vec<_>>())
        .await;
    Ok(result?)
}

async fn describe_schema(conn: &dyn Queryable, schemas: &[&str]) -> DescriberResult<SqlSchema> {
    let mut schema = SqlSchema::default();
    let mut ext = KingbaseOracleSchemaExt::default();
    if schemas.is_empty() {
        schema.set_connector_data(Box::new(ext));
        return Ok(schema);
    }
    let physical_schemas = schemas.iter().map(|name| normalize_namespace(name)).collect::<Vec<_>>();
    let schemas = physical_schemas.iter().map(|name| name.as_ref()).collect::<Vec<_>>();
    let schemas = schemas.as_slice();
    for row in query(conn, include_str!("kingbase_oracle/namespaces_query.sql"), schemas).await? {
        schema.push_namespace(row.get_expect_string("name"));
    }
    let mut tables = HashMap::new();
    let mut views = HashMap::new();
    for row in query(conn, OBJECTS, schemas).await? {
        let Some(ns) = schema.get_namespace_id(&row.get_expect_string("namespace")) else {
            continue;
        };
        let oid = row.get_expect_i64("object_id");
        if row.get_expect_string("kind") == "TABLE" {
            let id = schema.push_table(row.get_expect_string("name"), ns, row.get_string("description"));
            tables.insert(oid, id);
        } else {
            let id = schema.push_view(
                row.get_expect_string("name"),
                ns,
                row.get_string("definition"),
                row.get_string("description"),
            );
            views.insert(oid, id);
        }
    }
    let mut enums = HashMap::new();
    for row in query(conn, ENUMS, schemas).await? {
        let ns = schema.get_namespace_id(&row.get_expect_string("namespace")).unwrap();
        let name = row.get_expect_string("name");
        let id = *enums
            .entry((ns, name.clone()))
            .or_insert_with(|| schema.push_enum(ns, name, row.get_string("description")));
        schema.push_enum_variant(id, row.get_expect_string("value"));
    }
    for row in query(conn, COLUMNS, schemas).await? {
        let oid = row.get_expect_i64("object_id");
        if !tables.contains_key(&oid) && !views.contains_key(&oid) {
            continue;
        }
        let name = row.get_expect_string("type_name");
        let arity = if row.get_expect_bool("required") {
            ColumnArity::Required
        } else {
            ColumnArity::Nullable
        };
        let enum_id = schema
            .get_namespace_id(&row.get_expect_string("type_namespace"))
            .and_then(|ns| enums.get(&(ns, name.clone())).copied());
        let (family, native) = if row.get_expect_bool("is_array") {
            (ColumnTypeFamily::Unsupported(name.clone()), None)
        } else if let Some(id) = enum_id {
            (ColumnTypeFamily::Enum(id), None)
        } else {
            column_type(
                &name,
                row.get_i64("numeric_precision"),
                row.get_i64("numeric_scale"),
                row.get_i64("length"),
                row.get_i64("time_precision"),
            )
        };
        let default = row.get_string("default_expression").and_then(|s| {
            if enum_id.is_some() {
                default::parse_enum(&s, &family, &row.get_expect_string("type_namespace"), &name)
            } else {
                default::parse_native(&s, &family, native.as_ref())
            }
        });
        let identity = !row.get_expect_string("identity").is_empty();
        let auto_increment = identity
            || default
                .as_ref()
                .is_some_and(|d| matches!(d.kind(), DefaultKind::Sequence(_)));
        let column = Column {
            name: row.get_expect_string("name"),
            tpe: ColumnType {
                full_data_type: name,
                family,
                arity,
                native_type: native.map(NativeTypeInstance::new::<KingbaseOracleType>),
            },
            auto_increment,
            description: row.get_string("description"),
        };
        if let Some(&table) = tables.get(&oid) {
            let id = schema.push_table_column(table, column);
            if identity {
                let sequence = if let Some(sequence_oid) = row.get_i64("owned_sequence_oid") {
                    Some(
                        identity_sequence(
                            conn,
                            sequence_oid,
                            row.get_expect_string("owned_sequence_namespace"),
                            row.get_expect_string("owned_sequence_name"),
                        )
                        .await?,
                    )
                } else {
                    None
                };
                ext.column_generations
                    .insert(id, ColumnGeneration::Identity { sequence });
            } else if let (Some(owned_oid), Some(reference)) = (
                row.get_i64("owned_sequence_oid"),
                default.as_ref().and_then(|d| match d.kind() {
                    DefaultKind::Sequence(name) => Some(name.as_str()),
                    _ => None,
                }),
            ) {
                // Resolve the decompiler's reference through a bound value, not
                // by comparing strings or assuming it lives beside the table.
                let referenced = conn
                    .query_raw(
                        include_str!("kingbase_oracle/sequence_reference_query.sql"),
                        &[Value::text(reference)],
                    )
                    .await?;
                if referenced
                    .into_iter()
                    .next()
                    .is_some_and(|r| r.get_i64("oid") == Some(owned_oid))
                {
                    ext.column_generations.insert(
                        id,
                        ColumnGeneration::OwnedSequence {
                            namespace: row.get_expect_string("owned_sequence_namespace"),
                            name: row.get_expect_string("owned_sequence_name"),
                            shared: row.get_expect_bool("sequence_shared"),
                        },
                    );
                }
            }
            if let Some(value) = default {
                schema.push_table_default_value(id, value);
            }
        } else if let Some(&view) = views.get(&oid) {
            let id = schema.push_view_column(view, column);
            if let Some(value) = default {
                schema.push_view_default_value(id, value);
            }
        }
    }
    let mut indexes = HashMap::new();
    let mut expressions = std::collections::HashSet::new();
    let rows: Vec<_> = query(conn, INDEXES, schemas).await?.into_iter().collect();
    // Never expose part of a compound expression index as an ordinary index.
    for row in rows.iter() {
        if row.get_string("column_name").is_none() {
            expressions.insert((row.get_expect_i64("object_id"), row.get_expect_string("name")));
        }
    }
    for row in rows {
        let oid = row.get_expect_i64("object_id");
        let Some(&table) = tables.get(&oid) else { continue };
        let name = row.get_expect_string("name");
        if expressions.contains(&(oid, name.clone())) {
            continue;
        }
        let Some(column) = schema
            .walk(table)
            .column(&row.get_expect_string("column_name"))
            .map(|c| c.id)
        else {
            continue;
        };
        let index = *indexes.entry((oid, name.clone())).or_insert_with(|| {
            let id = if row.get_expect_bool("primary_key") {
                schema.push_primary_key(table, name)
            } else if row.get_expect_bool("is_unique") {
                match row.get_string("predicate") {
                    Some(p) => schema.push_partial_unique_constraint(table, name, p),
                    None => schema.push_unique_constraint(table, name),
                }
            } else {
                match row.get_string("predicate") {
                    Some(p) => schema.push_partial_index(table, name, p),
                    None => schema.push_index(table, name),
                }
            };
            if let Some(owner) = row.get_string("constraint_name") {
                ext.owning_constraint_names.insert(id, owner);
            }
            id
        });
        schema.push_index_column(IndexColumn {
            index_id: index,
            column_id: column,
            sort_order: Some(if row.get_bool("descending") == Some(true) {
                SQLSortOrder::Desc
            } else {
                SQLSortOrder::Asc
            }),
            length: None,
        });
    }
    for row in query(conn, CHECKS, schemas).await? {
        if let Some(&table) = tables.get(&row.get_expect_i64("object_id")) {
            schema.check_constraints.push((table, row.get_expect_string("name")));
        }
    }
    let mut fks = HashMap::new();
    for row in query(conn, FOREIGN_KEYS, schemas).await? {
        let Some(&table) = tables.get(&row.get_expect_i64("object_id")) else {
            continue;
        };
        let referenced = row.get_expect_i64("referenced_id");
        let Some(&target) = tables.get(&referenced) else {
            let ns = row.get_expect_string("referenced_namespace");
            if !schemas.contains(&ns.as_str()) {
                return Err(DescriberError::from(DescriberErrorKind::CrossSchemaReference {
                    from: format!(
                        "{}.{}",
                        schema.walk(table).namespace().unwrap_or(""),
                        schema.walk(table).name()
                    ),
                    to: format!("{}.{}", ns, row.get_expect_string("referenced_table")),
                    constraint: row.get_expect_string("name"),
                    missing_namespace: ns,
                }));
            }
            continue;
        };
        let Some(child) = schema
            .walk(table)
            .column(&row.get_expect_string("column_name"))
            .map(|c| c.id)
        else {
            continue;
        };
        let Some(parent) = schema
            .walk(target)
            .column(&row.get_expect_string("referenced_column"))
            .map(|c| c.id)
        else {
            continue;
        };
        let id = *fks.entry(row.get_expect_i64("constraint_id")).or_insert_with(|| {
            schema.push_foreign_key(
                Some(row.get_expect_string("name")),
                [table, target],
                [
                    action(&row.get_expect_string("on_delete")),
                    action(&row.get_expect_string("on_update")),
                ],
            )
        });
        ext.foreign_key_definitions
            .insert(id, row.get_expect_string("definition"));
        schema.push_foreign_key_column(id, [child, parent]);
    }
    for row in query(conn, PROCEDURES, schemas).await? {
        if let Some(namespace_id) = schema.get_namespace_id(&row.get_expect_string("namespace")) {
            schema.procedures.push(Procedure {
                namespace_id,
                name: row.get_expect_string("name"),
                definition: row.get_string("definition"),
            });
        }
    }
    schema.set_connector_data(Box::new(ext));
    Ok(schema)
}

fn action(action: &str) -> ForeignKeyAction {
    match action {
        "CASCADE" => ForeignKeyAction::Cascade,
        "RESTRICT" => ForeignKeyAction::Restrict,
        "SET NULL" => ForeignKeyAction::SetNull,
        "SET DEFAULT" => ForeignKeyAction::SetDefault,
        _ => ForeignKeyAction::NoAction,
    }
}

fn column_type(
    name: &str,
    precision: Option<i64>,
    scale: Option<i64>,
    length: Option<i64>,
    time: Option<i64>,
) -> (ColumnTypeFamily, Option<KingbaseOracleType>) {
    use KingbaseOracleType::*;
    let positive = |value: Option<i64>| value.and_then(|v| u32::try_from(v).ok());
    let number = |p: u32| Number(KingbaseOracleNumberArguments::PrecisionAndScale(p, 0));
    let native = match name.to_ascii_lowercase().as_str() {
        "number" | "numeric" | "decimal" => Number(match (positive(precision), positive(scale)) {
            (Some(p), Some(s)) => KingbaseOracleNumberArguments::PrecisionAndScale(p.max(s), s),
            (Some(p), None) => KingbaseOracleNumberArguments::Precision(p),
            _ => KingbaseOracleNumberArguments::Unspecified,
        }),
        "int2" | "int4" | "integer" | "smallint" => number(10),
        "int8" | "bigint" => number(19),
        "tinyint" => TinyInt,
        "float4" | "binary_float" => BinaryFloat,
        "float8" | "binary_double" => BinaryDouble,
        "bool" | "boolean" => Boolean,
        "bpchar" | "char" => Char(positive(length)),
        "varchar" | "varchar2" => VarChar2(positive(length)),
        "nchar" => NChar(positive(length)),
        "nvarchar2" => NVarChar2(positive(length)),
        "text" | "clob" => Clob,
        "nclob" => NClob,
        "bytea" | "blob" => Blob,
        "date" => Date,
        "timestamp" => Timestamp(positive(time)),
        "timestamptz" => TimestampTz(positive(time)),
        "timestamp_local_tz" => TimestampLocalTz(positive(time)),
        "json" | "jsonb" => Json,
        "uuid" => Uuid,
        "xml" => Xml,
        _ => return (ColumnTypeFamily::Unsupported(name.to_owned()), None),
    };
    let family = match native {
        TinyInt => ColumnTypeFamily::Int,
        Number(
            KingbaseOracleNumberArguments::Precision(p) | KingbaseOracleNumberArguments::PrecisionAndScale(p, 0),
        ) if p <= 10 => ColumnTypeFamily::Int,
        Number(
            KingbaseOracleNumberArguments::Precision(p) | KingbaseOracleNumberArguments::PrecisionAndScale(p, 0),
        ) if p <= 19 => ColumnTypeFamily::BigInt,
        Number(_) => ColumnTypeFamily::Decimal,
        BinaryFloat | BinaryDouble | Float(_) => ColumnTypeFamily::Float,
        Boolean => ColumnTypeFamily::Boolean,
        Blob => ColumnTypeFamily::Binary,
        Json => ColumnTypeFamily::Json,
        Date | Timestamp(_) | TimestampTz(_) | TimestampLocalTz(_) => ColumnTypeFamily::DateTime,
        _ => ColumnTypeFamily::String,
    };
    (family, Some(native))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn identifiers_preserve_mixed_case() {
        for (input, expected) in [("UPPER", "upper"), ("Mixed", "Mixed"), ("lower", "lower")] {
            let mut name = input.to_owned();
            normalize_identifier(&mut name);
            assert_eq!(name, expected);
        }
    }

    #[test]
    fn namespace_aliases_keep_object_namespace_ids() {
        let mut schema = SqlSchema::default();
        let upper = schema.push_namespace("APP".into());
        let lower = schema.push_namespace("app".into());
        let mixed = schema.push_namespace("App".into());
        schema.set_default_namespace("APP".into());
        schema.push_table("UpperTable".into(), upper, None);
        schema.push_table("LowerTable".into(), lower, None);
        schema.push_table("MixedTable".into(), mixed, None);
        schema.push_enum(mixed, "State".into(), None);
        schema.push_view("MixedView".into(), mixed, None, None);
        schema.push_udt(mixed, "CustomType".into(), None);
        schema.procedures.push(crate::Procedure {
            namespace_id: mixed,
            name: "routine".into(),
            definition: None,
        });

        normalize_schema_identifiers(&mut schema);

        assert_eq!(
            schema.namespaces.iter().map(String::as_str).collect::<Vec<_>>(),
            ["app", "App"]
        );
        assert_eq!(schema.default_namespace(), Some("app"));
        assert!(schema.table_walker_ns("app", "UpperTable").is_some());
        assert!(schema.table_walker_ns("app", "LowerTable").is_some());
        assert!(schema.table_walker_ns("App", "MixedTable").is_some());
        let mixed = schema.get_namespace_id("App").unwrap();
        assert_eq!(schema.enums[0].namespace_id, mixed);
        assert_eq!(schema.views[0].namespace_id, mixed);
        assert_eq!(schema.user_defined_types[0].namespace_id, mixed);
        assert_eq!(schema.procedures[0].namespace_id, mixed);

        preserve_namespace_spelling(&mut schema, &["APP".into(), "App".into()]);
        assert!(schema.table_walker_ns("APP", "UpperTable").is_some());
        assert!(schema.table_walker_ns("App", "MixedTable").is_some());
    }

    #[test]
    fn namespace_free_schema_still_normalizes_identifiers() {
        let mut schema = SqlSchema::default();
        schema.push_table("UPPER".into(), Default::default(), None);
        normalize_schema_identifiers(&mut schema);
        assert_eq!(schema.tables[0].name, "upper");
    }

    #[test]
    fn number_one_is_numeric_and_scale_above_precision_is_canonical() {
        assert_eq!(
            column_type("numeric", Some(1), Some(0), None, None).0,
            ColumnTypeFamily::Int
        );
        assert_eq!(
            column_type("NUMBER", Some(2), Some(3), None, None).1,
            Some(KingbaseOracleType::Number(
                KingbaseOracleNumberArguments::PrecisionAndScale(3, 3)
            ))
        );
        assert_eq!(column_type("blob", None, None, None, None).0, ColumnTypeFamily::Binary);
    }
    #[test]
    fn metadata_templates_are_explicit_and_use_bound_namespace_filters() {
        for sql in [OBJECTS, COLUMNS, ENUMS, INDEXES, FOREIGN_KEYS, CHECKS, PROCEDURES] {
            assert!(sql.contains("__SCHEMAS__"));
            assert!(!sql.contains("pg_catalog") && !sql.contains("FROM pg_") && !sql.contains("JOIN pg_"));
        }
        assert!(OBJECTS.contains("ALL_OBJECTS"));
        assert!(INDEXES.contains("sys_index_column_has_property"));
    }
}
