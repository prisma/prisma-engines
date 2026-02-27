//! Postgres schema description.

mod default;
mod extensions;

use either::Either;
pub use extensions::{DatabaseExtension, ExtensionId, ExtensionWalker};

use self::default::get_default_value;
use super::*;
use crate::getters::Getter;
use enumflags2::BitFlags;
use indexmap::IndexMap;
use indoc::indoc;
use psl::{
    builtin_connectors::{CockroachType, KnownPostgresType, PostgresType},
    datamodel_connector::NativeTypeInstance,
};
use quaint::{Value, connector::ResultRow, prelude::Queryable};
use regex::Regex;
use std::{
    any::type_name,
    collections::{BTreeMap, HashMap},
    iter::Peekable,
    sync::LazyLock,
};
use tracing::trace;

/// A PostgreSQL sequence.
/// <https://www.postgresql.org/docs/current/view-pg-sequences.html>
#[derive(Debug, Clone)]
pub struct Sequence {
    /// Sequence name
    pub namespace_id: NamespaceId,
    /// Sequence name
    pub name: String,
    /// Start value of the sequence
    pub start_value: i64,
    /// Minimum value of the sequence
    pub min_value: i64,
    /// Maximum value of the sequence
    pub max_value: i64,
    /// Increment value of the sequence
    pub increment_by: i64,
    /// Whether the sequence cycles
    pub cycle: bool,
    /// Cache size of the sequence
    pub cache_size: i64,
    /// Whether the sequence is a cockroachdb virtual sequence
    pub r#virtual: bool,
}

// We impl default manually to align with database defaults.
impl Default for Sequence {
    fn default() -> Self {
        Sequence {
            namespace_id: NamespaceId::default(),
            name: String::default(),
            start_value: 1,
            min_value: 1,
            max_value: i64::MAX,
            increment_by: 1,
            cycle: false,
            cache_size: 1,
            r#virtual: false,
        }
    }
}

#[derive(PartialEq, Debug, Clone, Copy, Default)]
pub enum SqlIndexAlgorithm {
    #[default]
    BTree,
    Hash,
    Gist,
    Gin,
    SpGist,
    Brin,
}

impl AsRef<str> for SqlIndexAlgorithm {
    fn as_ref(&self) -> &str {
        match self {
            Self::BTree => "BTREE",
            Self::Hash => "HASH",
            Self::Gist => "GIST",
            Self::Gin => "GIN",
            Self::SpGist => "SPGIST",
            Self::Brin => "BRIN",
        }
    }
}

impl fmt::Display for SqlIndexAlgorithm {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_ref())
    }
}

#[enumflags2::bitflags]
#[derive(Clone, Copy, Debug)]
#[repr(u8)]
pub enum Circumstances {
    Cockroach,
    CockroachWithPostgresNativeTypes, // TODO: this is a temporary workaround
    CanPartitionTables,
}

pub struct SqlSchemaDescriber<'a> {
    conn: &'a dyn Queryable,
    circumstances: BitFlags<Circumstances>,
}

impl Debug for SqlSchemaDescriber<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct(type_name::<SqlSchemaDescriber<'_>>())
            .field("circumstances", &self.circumstances)
            .finish()
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum IndexNullPosition {
    First,
    Last,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Constraint {
    Index(IndexId),
    ForeignKey(ForeignKeyId),
}

#[enumflags2::bitflags]
#[derive(Clone, Copy, Debug)]
#[repr(u8)]
pub enum ConstraintOption {
    Deferred,
    Deferrable,
}

#[derive(Default, Debug, Clone)]
pub struct PostgresSchemaExt {
    pub opclasses: Vec<(IndexColumnId, SQLOperatorClass)>,
    pub indexes: Vec<(IndexId, SqlIndexAlgorithm)>,
    pub expression_indexes: Vec<(TableId, String)>,
    pub index_null_position: HashMap<IndexColumnId, IndexNullPosition>,
    pub constraint_options: HashMap<Constraint, BitFlags<ConstraintOption>>,
    pub table_options: Vec<BTreeMap<String, String>>,
    pub exclude_constraints: Vec<(TableId, String)>,
    /// The schema's sequences.
    pub sequences: Vec<Sequence>,
    /// The extensions included in the schema(s).
    extensions: Vec<DatabaseExtension>,
}

impl PostgresSchemaExt {
    #[track_caller]
    pub fn index_algorithm(&self, index_id: IndexId) -> SqlIndexAlgorithm {
        match self.indexes.binary_search_by_key(&index_id, |(id, _)| *id) {
            Ok(i) => self.indexes[i].1,
            Err(_) => Default::default(),
        }
    }

    pub fn get_opclass(&self, index_field_id: IndexColumnId) -> Option<&SQLOperatorClass> {
        let idx = self
            .opclasses
            .binary_search_by_key(&index_field_id, |(id, _)| *id)
            .ok()?;
        Some(&self.opclasses[idx].1)
    }

    pub fn get_sequence(&self, name: &str) -> Option<(usize, &Sequence)> {
        self.sequences
            .binary_search_by_key(&name, |s| &s.name)
            .map(|idx| (idx, &self.sequences[idx]))
            .ok()
    }

    pub fn extension_walkers(&self) -> impl Iterator<Item = ExtensionWalker<'_>> {
        (0..self.extensions.len()).map(move |idx| ExtensionWalker {
            schema_ext: self,
            id: ExtensionId(idx as u32),
        })
    }

    pub fn extension_walker<'a>(&'a self, name: &str) -> Option<ExtensionWalker<'a>> {
        self.extension_walkers().find(|ext| ext.name() == name)
    }

    pub fn push_extension(&mut self, extension: DatabaseExtension) {
        self.extensions.push(extension);
    }

    pub fn get_extension(&self, id: ExtensionId) -> &DatabaseExtension {
        &self.extensions[id.0 as usize]
    }

    pub fn clear_extensions(&mut self) {
        self.extensions.clear();
    }

    pub fn non_default_null_position(&self, column: IndexColumnWalker<'_>) -> bool {
        let position = self.index_null_position.get(&column.id);

        let position = match position {
            Some(position) => *position,
            _ => return false,
        };

        let sort_order = match column.sort_order() {
            Some(order) => order,
            _ => return false,
        };

        match sort_order {
            SQLSortOrder::Asc => position == IndexNullPosition::First,
            SQLSortOrder::Desc => position == IndexNullPosition::Last,
        }
    }

    pub fn uses_row_level_ttl(&self, id: TableId) -> bool {
        let options = match self.table_options.get(id.0 as usize) {
            Some(options) => options,
            None => return false,
        };

        match options.get("ttl") {
            Some(val) => val.as_str() == "'on'",
            None => false,
        }
    }

    pub fn non_default_foreign_key_constraint_deferring(&self, id: ForeignKeyId) -> bool {
        match self.constraint_options.get(&Constraint::ForeignKey(id)) {
            Some(opts) => opts.contains(ConstraintOption::Deferrable) || opts.contains(ConstraintOption::Deferred),
            None => false,
        }
    }

    pub fn non_default_index_constraint_deferring(&self, id: IndexId) -> bool {
        match self.constraint_options.get(&Constraint::Index(id)) {
            Some(opts) => opts.contains(ConstraintOption::Deferrable) || opts.contains(ConstraintOption::Deferred),
            None => false,
        }
    }

    pub fn exclude_constraints(&self, table_id: TableId) -> impl ExactSizeIterator<Item = &str> {
        let low = self.exclude_constraints.partition_point(|(id, _)| *id < table_id);
        let high = self.exclude_constraints[low..].partition_point(|(id, _)| *id <= table_id);

        self.exclude_constraints[low..low + high]
            .iter()
            .map(|(_, name)| name.as_str())
    }

    pub fn uses_exclude_constraint(&self, id: TableId) -> bool {
        self.exclude_constraints
            .binary_search_by_key(&id, |(id, _)| *id)
            .is_ok()
    }
}

#[derive(Clone, Debug)]
pub struct SQLOperatorClass {
    pub kind: SQLOperatorClassKind,
    pub is_default: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub enum SQLOperatorClassKind {
    /// GiST + inet type
    InetOps,
    /// GIN + jsonb type
    JsonbOps,
    /// GIN + jsonb type
    JsonbPathOps,
    /// GIN + array type
    ArrayOps,
    /// SP-GiST + text type
    TextOps,
    /// BRIN + bit
    BitMinMaxOps,
    /// BRIN + varbit
    VarBitMinMaxOps,
    /// BRIN + char
    BpcharBloomOps,
    /// BRIN + char
    BpcharMinMaxOps,
    /// BRIN + bytea
    ByteaBloomOps,
    /// BRIN + bytea
    ByteaMinMaxOps,
    /// BRIN + date
    DateBloomOps,
    /// BRIN + date
    DateMinMaxOps,
    /// BRIN + date
    DateMinMaxMultiOps,
    /// BRIN + float
    Float4BloomOps,
    /// BRIN + float
    Float4MinMaxOps,
    /// BRIN + float
    Float4MinMaxMultiOps,
    /// BRIN + double
    Float8BloomOps,
    /// BRIN + double
    Float8MinMaxOps,
    /// BRIN + double
    Float8MinMaxMultiOps,
    /// BRIN + inet
    InetInclusionOps,
    /// BRIN + inet
    InetBloomOps,
    /// BRIN + inet
    InetMinMaxOps,
    /// BRIN + inet
    InetMinMaxMultiOps,
    /// BRIN + int2
    Int2BloomOps,
    /// BRIN + int2
    Int2MinMaxOps,
    /// BRIN + int2
    Int2MinMaxMultiOps,
    /// BRIN + int4
    Int4BloomOps,
    /// BRIN + int4
    Int4MinMaxOps,
    /// BRIN + int4
    Int4MinMaxMultiOps,
    /// BRIN + int8
    Int8BloomOps,
    /// BRIN + int8
    Int8MinMaxOps,
    /// BRIN + int8
    Int8MinMaxMultiOps,
    /// BRIN + numeric
    NumericBloomOps,
    /// BRIN + numeric
    NumericMinMaxOps,
    /// BRIN + numeric
    NumericMinMaxMultiOps,
    /// BRIN + oid
    OidBloomOps,
    /// BRIN + oid
    OidMinMaxOps,
    /// BRIN + oid
    OidMinMaxMultiOps,
    /// BRIN + text
    TextBloomOps,
    /// BRIN + text
    TextMinMaxOps,
    /// BRIN + timestamp
    TimestampBloomOps,
    /// BRIN + timestamp
    TimestampMinMaxOps,
    /// BRIN + timestamp
    TimestampMinMaxMultiOps,
    /// BRIN + timestamptz
    TimestampTzBloomOps,
    /// BRIN + timestamptz
    TimestampTzMinMaxOps,
    /// BRIN + timestamptz
    TimestampTzMinMaxMultiOps,
    /// BRIN + time
    TimeBloomOps,
    /// BRIN + time
    TimeMinMaxOps,
    /// BRIN + time
    TimeMinMaxMultiOps,
    /// BRIN + timetz
    TimeTzBloomOps,
    /// BRIN + timetz
    TimeTzMinMaxOps,
    /// BRIN + timetz
    TimeTzMinMaxMultiOps,
    /// BRIN + uuid
    UuidBloomOps,
    /// BRIN + uuid
    UuidMinMaxOps,
    /// BRIN + uuid
    UuidMinMaxMultiOps,

    /// Escape hatch
    Raw(String),
}

impl SQLOperatorClassKind {
    pub fn raw(s: &str) -> Self {
        Self::Raw(s.to_string())
    }
}

impl From<&str> for SQLOperatorClassKind {
    fn from(s: &str) -> Self {
        match s {
            "array_ops" => SQLOperatorClassKind::ArrayOps,
            "jsonb_ops" => SQLOperatorClassKind::JsonbOps,
            "text_ops" => SQLOperatorClassKind::TextOps,
            "bit_minmax_ops" => SQLOperatorClassKind::BitMinMaxOps,
            "varbit_minmax_ops" => SQLOperatorClassKind::VarBitMinMaxOps,
            "bpchar_minmax_ops" => SQLOperatorClassKind::BpcharMinMaxOps,
            "bytea_minmax_ops" => SQLOperatorClassKind::ByteaMinMaxOps,
            "float4_minmax_ops" => SQLOperatorClassKind::Float4MinMaxOps,
            "date_minmax_ops" => SQLOperatorClassKind::DateMinMaxOps,
            "float8_minmax_ops" => SQLOperatorClassKind::Float8MinMaxOps,
            "inet_inclusion_ops" => SQLOperatorClassKind::InetInclusionOps,
            "int2_minmax_ops" => SQLOperatorClassKind::Int2MinMaxOps,
            "int4_minmax_ops" => SQLOperatorClassKind::Int4MinMaxOps,
            "int8_minmax_ops" => SQLOperatorClassKind::Int8MinMaxOps,
            "numeric_minmax_ops" => SQLOperatorClassKind::NumericMinMaxOps,
            "oid_minmax_ops" => SQLOperatorClassKind::OidMinMaxOps,
            "text_minmax_ops" => SQLOperatorClassKind::TextMinMaxOps,
            "timestamp_minmax_ops" => SQLOperatorClassKind::TimestampMinMaxOps,
            "timestamptz_minmax_ops" => SQLOperatorClassKind::TimestampTzMinMaxOps,
            "time_minmax_ops" => SQLOperatorClassKind::TimeMinMaxOps,
            "timetz_minmax_ops" => SQLOperatorClassKind::TimeTzMinMaxOps,
            "uuid_minmax_ops" => SQLOperatorClassKind::UuidMinMaxOps,
            "inet_ops" => SQLOperatorClassKind::InetOps,
            "jsonb_path_ops" => SQLOperatorClassKind::JsonbPathOps,
            "bpchar_bloom_ops" => SQLOperatorClassKind::BpcharBloomOps,
            "bytea_bloom_ops" => SQLOperatorClassKind::ByteaBloomOps,
            "date_bloom_ops" => SQLOperatorClassKind::DateBloomOps,
            "date_minmax_multi_ops" => SQLOperatorClassKind::DateMinMaxMultiOps,
            "float4_bloom_ops" => SQLOperatorClassKind::Float4BloomOps,
            "float4_minmax_multi_ops" => SQLOperatorClassKind::Float4MinMaxMultiOps,
            "float8_bloom_ops" => SQLOperatorClassKind::Float8BloomOps,
            "float8_minmax_multi_ops" => SQLOperatorClassKind::Float8MinMaxMultiOps,
            "inet_bloom_ops" => SQLOperatorClassKind::InetBloomOps,
            "inet_minmax_ops" => SQLOperatorClassKind::InetMinMaxOps,
            "inet_minmax_multi_ops" => SQLOperatorClassKind::InetMinMaxMultiOps,
            "int2_bloom_ops" => SQLOperatorClassKind::Int2BloomOps,
            "int2_minmax_multi_ops" => SQLOperatorClassKind::Int2MinMaxMultiOps,
            "int4_bloom_ops" => SQLOperatorClassKind::Int4BloomOps,
            "int4_minmax_multi_ops" => SQLOperatorClassKind::Int4MinMaxMultiOps,
            "int8_bloom_ops" => SQLOperatorClassKind::Int8BloomOps,
            "int8_minmax_multi_ops" => SQLOperatorClassKind::Int8MinMaxMultiOps,
            "numeric_bloom_ops" => SQLOperatorClassKind::NumericBloomOps,
            "numeric_minmax_multi_ops" => SQLOperatorClassKind::NumericMinMaxMultiOps,
            "oid_bloom_ops" => SQLOperatorClassKind::OidBloomOps,
            "oid_minmax_multi_ops" => SQLOperatorClassKind::OidMinMaxMultiOps,
            "text_bloom_ops" => SQLOperatorClassKind::TextBloomOps,
            "timestamp_bloom_ops" => SQLOperatorClassKind::TimestampBloomOps,
            "timestamp_minmax_multi_ops" => SQLOperatorClassKind::TimestampMinMaxMultiOps,
            "timestamptz_bloom_ops" => SQLOperatorClassKind::TimestampTzBloomOps,
            "timestamptz_minmax_multi_ops" => SQLOperatorClassKind::TimestampTzMinMaxMultiOps,
            "time_bloom_ops" => SQLOperatorClassKind::TimeBloomOps,
            "time_minmax_multi_ops" => SQLOperatorClassKind::TimeMinMaxMultiOps,
            "timetz_bloom_ops" => SQLOperatorClassKind::TimeTzBloomOps,
            "timetz_minmax_multi_ops" => SQLOperatorClassKind::TimeTzMinMaxMultiOps,
            "uuid_bloom_ops" => SQLOperatorClassKind::UuidBloomOps,
            "uuid_minmax_multi_ops" => SQLOperatorClassKind::UuidMinMaxMultiOps,
            _ => SQLOperatorClassKind::Raw(s.to_string()),
        }
    }
}

impl AsRef<str> for SQLOperatorClassKind {
    fn as_ref(&self) -> &str {
        match self {
            SQLOperatorClassKind::InetOps => "inet_ops",
            SQLOperatorClassKind::JsonbOps => "jsonb_ops",
            SQLOperatorClassKind::JsonbPathOps => "jsonb_path_ops",
            SQLOperatorClassKind::ArrayOps => "array_ops",
            SQLOperatorClassKind::TextOps => "text_ops",
            SQLOperatorClassKind::BitMinMaxOps => "bit_minmax_ops",
            SQLOperatorClassKind::VarBitMinMaxOps => "varbit_minmax_ops",
            SQLOperatorClassKind::BpcharBloomOps => "bpchar_bloom_ops",
            SQLOperatorClassKind::BpcharMinMaxOps => "bpchar_minmax_ops",
            SQLOperatorClassKind::ByteaBloomOps => "bytea_bloom_ops",
            SQLOperatorClassKind::ByteaMinMaxOps => "bytea_minmax_ops",
            SQLOperatorClassKind::DateBloomOps => "date_bloom_ops",
            SQLOperatorClassKind::DateMinMaxOps => "date_minmax_ops",
            SQLOperatorClassKind::DateMinMaxMultiOps => "date_minmax_multi_ops",
            SQLOperatorClassKind::Float4BloomOps => "float4_bloom_ops",
            SQLOperatorClassKind::Float4MinMaxOps => "float4_minmax_ops",
            SQLOperatorClassKind::Float4MinMaxMultiOps => "float4_minmax_multi_ops",
            SQLOperatorClassKind::Float8BloomOps => "float8_bloom_ops",
            SQLOperatorClassKind::Float8MinMaxOps => "float8_minmax_ops",
            SQLOperatorClassKind::Float8MinMaxMultiOps => "float8_minmax_multi_ops",
            SQLOperatorClassKind::InetInclusionOps => "inet_inclusion_ops",
            SQLOperatorClassKind::InetBloomOps => "inet_bloom_ops",
            SQLOperatorClassKind::InetMinMaxOps => "inet_minmax_ops",
            SQLOperatorClassKind::InetMinMaxMultiOps => "inet_minmax_multi_ops",
            SQLOperatorClassKind::Int2BloomOps => "int2_bloom_ops",
            SQLOperatorClassKind::Int2MinMaxOps => "int2_minmax_ops",
            SQLOperatorClassKind::Int2MinMaxMultiOps => "int2_minmax_multi_ops",
            SQLOperatorClassKind::Int4BloomOps => "int4_bloom_ops",
            SQLOperatorClassKind::Int4MinMaxOps => "int4_minmax_ops",
            SQLOperatorClassKind::Int4MinMaxMultiOps => "int4_minmax_multi_ops",
            SQLOperatorClassKind::Int8BloomOps => "int8_bloom_ops",
            SQLOperatorClassKind::Int8MinMaxOps => "int8_minmax_ops",
            SQLOperatorClassKind::Int8MinMaxMultiOps => "int8_minmax_multi_ops",
            SQLOperatorClassKind::NumericBloomOps => "numeric_bloom_ops",
            SQLOperatorClassKind::NumericMinMaxOps => "numeric_minmax_ops",
            SQLOperatorClassKind::NumericMinMaxMultiOps => "numeric_minmax_multi_ops",
            SQLOperatorClassKind::OidBloomOps => "oid_bloom_ops",
            SQLOperatorClassKind::OidMinMaxOps => "oid_minmax_ops",
            SQLOperatorClassKind::OidMinMaxMultiOps => "oid_minmax_multi_ops",
            SQLOperatorClassKind::TextBloomOps => "text_bloom_ops",
            SQLOperatorClassKind::TextMinMaxOps => "text_minmax_ops",
            SQLOperatorClassKind::TimestampBloomOps => "timestamp_bloom_ops",
            SQLOperatorClassKind::TimestampMinMaxOps => "timestamp_minmax_ops",
            SQLOperatorClassKind::TimestampMinMaxMultiOps => "timestamp_minmax_multi_ops",
            SQLOperatorClassKind::TimestampTzBloomOps => "timestamptz_bloom_ops",
            SQLOperatorClassKind::TimestampTzMinMaxOps => "timestamptz_minmax_ops",
            SQLOperatorClassKind::TimestampTzMinMaxMultiOps => "timestamptz_minmax_multi_ops",
            SQLOperatorClassKind::TimeBloomOps => "time_bloom_ops",
            SQLOperatorClassKind::TimeMinMaxOps => "time_minmax_ops",
            SQLOperatorClassKind::TimeMinMaxMultiOps => "time_minmax_multi_ops",
            SQLOperatorClassKind::TimeTzBloomOps => "timetz_bloom_ops",
            SQLOperatorClassKind::TimeTzMinMaxOps => "timetz_minmax_ops",
            SQLOperatorClassKind::TimeTzMinMaxMultiOps => "timetz_minmax_multi_ops",
            SQLOperatorClassKind::UuidBloomOps => "uuid_bloom_ops",
            SQLOperatorClassKind::UuidMinMaxOps => "uuid_minmax_ops",
            SQLOperatorClassKind::UuidMinMaxMultiOps => "uuid_minmax_multi_ops",
            SQLOperatorClassKind::Raw(c) => c,
        }
    }
}

#[async_trait::async_trait]
impl super::SqlSchemaDescriberBackend for SqlSchemaDescriber<'_> {
    async fn describe(&self, schemas: &[&str]) -> DescriberResult<SqlSchema> {
        let mut sql_schema = SqlSchema::default();
        let mut pg_ext = PostgresSchemaExt::default();

        self.get_namespaces(&mut sql_schema, schemas).await?;

        //TODO(matthias) can we get rid of the table names map and instead just use tablewalker_ns everywhere like in get_columns?
        let table_names = self.get_table_names(&mut sql_schema, &mut pg_ext).await?;

        // order matters
        self.get_constraints(&table_names, &mut sql_schema, &mut pg_ext).await?;
        self.get_views(&mut sql_schema).await?;
        self.get_enums(&mut sql_schema).await?;
        self.get_udts(&mut sql_schema).await?;
        self.get_columns(&mut sql_schema).await?;
        self.get_foreign_keys(&table_names, &mut pg_ext, &mut sql_schema)
            .await?;
        self.get_indices(&table_names, &mut pg_ext, &mut sql_schema).await?;

        self.get_procedures(&mut sql_schema).await?;
        self.get_extensions(&mut pg_ext).await?;

        //Todo(matthias) understand this
        self.get_sequences(&sql_schema, &mut pg_ext).await?;
        // Make sure the vectors we use binary search on are sorted.
        pg_ext.indexes.sort_by_key(|(id, _)| *id);
        pg_ext.opclasses.sort_by_key(|(id, _)| *id);

        sql_schema.connector_data = connector_data::ConnectorData {
            data: Some(Box::new(pg_ext)),
        };

        Ok(sql_schema)
    }

    async fn version(&self) -> crate::DescriberResult<Option<String>> {
        Ok(self.conn.version().await?)
    }
}

impl<'a> SqlSchemaDescriber<'a> {
    pub fn new(conn: &'a dyn Queryable, circumstances: BitFlags<Circumstances>) -> SqlSchemaDescriber<'a> {
        SqlSchemaDescriber { conn, circumstances }
    }

    fn is_cockroach(&self) -> bool {
        self.circumstances.contains(Circumstances::Cockroach)
    }

    async fn get_extensions(&self, pg_ext: &mut PostgresSchemaExt) -> DescriberResult<()> {
        // CockroachDB does not support extensions.
        if self.is_cockroach() {
            return Ok(());
        }

        let sql = indoc! {r#"
            SELECT
                ext.extname AS extension_name,
                ext.extversion AS extension_version,
                ext.extrelocatable AS extension_relocatable,
                pn.nspname AS extension_schema
            FROM pg_extension ext
            INNER JOIN pg_namespace pn ON ext.extnamespace = pn.oid
            ORDER BY ext.extname ASC
        "#};

        let rows = self.conn.query_raw(sql, &[]).await?;
        let mut extensions = Vec::with_capacity(rows.len());

        for row in rows.into_iter() {
            extensions.push(DatabaseExtension {
                name: row.get_expect_string("extension_name"),
                schema: row.get_expect_string("extension_schema"),
                version: row.get_expect_string("extension_version"),
                relocatable: row.get_expect_bool("extension_relocatable"),
            });
        }

        pg_ext.extensions = extensions;

        Ok(())
    }

    async fn get_procedures(&self, sql_schema: &mut SqlSchema) -> DescriberResult<()> {
        let namespaces = &sql_schema.namespaces;

        if self.is_cockroach() {
            return Ok(());
        }

        let sql = r#"
            SELECT p.proname AS name, n.nspname as namespace,
                CASE WHEN l.lanname = 'internal' THEN p.prosrc
                     ELSE pg_get_functiondef(p.oid)
                     END as definition
            FROM pg_proc p
            LEFT JOIN pg_namespace n ON p.pronamespace = n.oid
            LEFT JOIN pg_language l ON p.prolang = l.oid
            WHERE n.nspname = ANY ( $1 )
        "#;

        let rows = self.conn.query_raw(sql, &[Value::array(namespaces)]).await?;

        let mut procedures = Vec::with_capacity(rows.len());

        for row in rows.into_iter() {
            procedures.push(Procedure {
                namespace_id: sql_schema
                    .get_namespace_id(&row.get_expect_string("namespace"))
                    .unwrap(),
                name: row.get_expect_string("name"),
                definition: row.get_string("definition"),
            });
        }

        sql_schema.procedures = procedures;

        Ok(())
    }

    async fn get_namespaces(&self, sql_schema: &mut SqlSchema, namespaces: &[&str]) -> DescriberResult<()> {
        // Although we have a list of namespaces we should introspect, we still need to check whether they actually already exist in the database.
        let sql = include_str!("postgres/namespaces_query.sql");

        let rows = self.conn.query_raw(sql, &[Value::array(namespaces)]).await?;

        let names = rows.into_iter().map(|row| row.get_expect_string("namespace_name"));

        for namespace in names {
            sql_schema.push_namespace(namespace);
        }

        Ok(())
    }

    async fn get_table_names(
        &self,
        sql_schema: &mut SqlSchema,
        pg_ext: &mut PostgresSchemaExt,
    ) -> DescriberResult<IndexMap<(String, String), TableId>> {
        let sql = if self.circumstances.contains(Circumstances::CanPartitionTables) {
            include_str!("postgres/tables_query.sql")
        } else {
            include_str!("postgres/tables_query_simple.sql")
        };

        let namespaces = &sql_schema.namespaces;

        let rows = self.conn.query_raw(sql, &[Value::array(namespaces)]).await?;

        let mut names = Vec::with_capacity(rows.len());

        for row in rows.into_iter() {
            let options: BTreeMap<String, String> = row
                .get_string_array("reloptions")
                .unwrap_or_default()
                .into_iter()
                .filter_map(|s| {
                    let mut splitted = s.split('=');
                    let key = splitted.next()?.to_string();
                    let value = splitted.next()?.to_string();

                    Some((key, value))
                })
                .collect();

            names.push((
                row.get_expect_string("table_name"),
                row.get_expect_string("namespace"),
                row.get_expect_bool("is_partition"),
                row.get_expect_bool("has_subclass"),
                row.get_expect_bool("has_row_level_security"),
                row.get_string("description"),
            ));

            pg_ext.table_options.push(options);
        }

        let mut map = IndexMap::default();

        for (table_name, namespace, is_partition, has_subclass, has_row_level_security, description) in names {
            let cloned_name = table_name.clone();

            let partition = if is_partition {
                BitFlags::from_flag(TableProperties::IsPartition)
            } else {
                BitFlags::empty()
            };

            let subclass = if has_subclass {
                BitFlags::from_flag(TableProperties::HasSubclass)
            } else {
                BitFlags::empty()
            };
            let row_level_security = if has_row_level_security {
                BitFlags::from_flag(TableProperties::HasRowLevelSecurity)
            } else {
                BitFlags::empty()
            };

            let constraints_key = (namespace.clone(), cloned_name);

            // TODO: we could build this without check and exclusion constraints.
            // We could then mutate the tables by id to add these constraints as "properties".
            // This would allow "get_constraints" to avoid using BTreeMaps in favor of mutating
            // the map in place.
            let id = sql_schema.push_table_with_properties(
                table_name,
                sql_schema.get_namespace_id(&namespace).unwrap(),
                partition | subclass | row_level_security,
                description,
            );

            map.insert(constraints_key, id);
        }

        Ok(map)
    }

    async fn get_views(&self, sql_schema: &mut SqlSchema) -> DescriberResult<()> {
        let namespaces = &sql_schema.namespaces;
        let sql = indoc! {r#"
            SELECT
                views.viewname AS view_name,
                views.definition AS view_sql,
                views.schemaname AS namespace,
                obj_description(class.oid, 'pg_class') AS description
            FROM pg_catalog.pg_views views
            INNER JOIN pg_catalog.pg_namespace ns ON views.schemaname = ns.nspname
            INNER JOIN pg_catalog.pg_class class ON class.relnamespace = ns.oid AND class.relname = views.viewname
            WHERE schemaname = ANY ( $1 )
        "#};

        let result_set = self.conn.query_raw(sql, &[Value::array(namespaces)]).await?;

        for row in result_set.into_iter() {
            let name = row.get_expect_string("view_name");
            let definition = row.get_string("view_sql");
            let description = row.get_string("description");

            let namespace_id = sql_schema
                .get_namespace_id(&row.get_expect_string("namespace"))
                .unwrap();

            sql_schema.push_view(name, namespace_id, definition, description);
        }

        Ok(())
    }

    async fn get_columns(&self, sql_schema: &mut SqlSchema) -> DescriberResult<()> {
        let namespaces = &sql_schema.namespaces;
        let mut table_defaults = Vec::new();
        let mut view_defaults = Vec::new();

        let is_visible_clause = if self.is_cockroach() {
            " AND info.is_hidden = 'NO'"
        } else {
            ""
        };

        let sql = format!(
            r#"
            SELECT
                oid.namespace,
                info.table_name,
                info.column_name,
                format_type(att.atttypid, att.atttypmod) as formatted_type,
                info.numeric_precision,
                info.numeric_scale,
                info.numeric_precision_radix,
                info.datetime_precision,
                info.data_type,
                info.udt_schema as type_schema_name,
                info.udt_name as full_data_type,
                pg_get_expr(attdef.adbin, attdef.adrelid) AS column_default,
                info.is_nullable,
                info.is_identity,
                info.character_maximum_length,
                col_description(att.attrelid, ordinal_position) AS description
            FROM information_schema.columns info
            JOIN pg_attribute att ON att.attname = info.column_name
            JOIN (
                 SELECT pg_class.oid, relname, pg_namespace.nspname as namespace
                 FROM pg_class
                 JOIN pg_namespace on pg_namespace.oid = pg_class.relnamespace
                 AND pg_namespace.nspname = ANY ( $1 )
                 WHERE reltype > 0
                ) as oid on oid.oid = att.attrelid
                  AND relname = info.table_name
                  AND namespace = info.table_schema
            LEFT OUTER JOIN pg_attrdef attdef ON attdef.adrelid = att.attrelid AND attdef.adnum = att.attnum AND table_schema = namespace
            WHERE table_schema = ANY ( $1 ) {is_visible_clause}
            ORDER BY namespace, table_name, ordinal_position;
        "#
        );

        let rows = self.conn.query_raw(sql.as_str(), &[Value::array(namespaces)]).await?;

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

            let is_identity = match col.get_string("is_identity") {
                Some(is_id) if is_id.eq_ignore_ascii_case("yes") => true,
                Some(is_id) if is_id.eq_ignore_ascii_case("no") => false,
                Some(is_identity_str) => panic!("unrecognized is_identity variant '{is_identity_str}'"),
                None => false,
            };

            let tpe = if self.is_cockroach()
                && !self
                    .circumstances
                    .contains(Circumstances::CockroachWithPostgresNativeTypes)
            {
                get_column_type_cockroachdb(&col, sql_schema)
            } else {
                get_column_type_postgresql(&col, sql_schema)
            };

            let default = col
                .get("column_default")
                .and_then(|raw_default_value| raw_default_value.to_string())
                .and_then(|raw_default_value| get_default_value(&raw_default_value, &tpe));

            let description = col.get_string("description");

            let auto_increment = is_identity
                || matches!(default.as_ref().map(|d| &d.kind), Some(DefaultKind::Sequence(_)))
                || (self.is_cockroach()
                    && matches!(
                        default.as_ref().map(|d| &d.kind),
                        Some(DefaultKind::DbGenerated(Some(s))) if s == "unique_rowid()"
                    ));

            match container_id {
                Either::Left(table_id) => {
                    table_defaults.push((table_id, default));
                }
                Either::Right(view_id) => {
                    view_defaults.push((view_id, default));
                }
            }

            let col = Column {
                name,
                tpe,
                auto_increment,
                description,
            };

            match container_id {
                Either::Left(table_id) => {
                    sql_schema.push_table_column(table_id, col);
                }
                Either::Right(view_id) => {
                    sql_schema.push_view_column(view_id, col);
                }
            }
        }

        // We need to sort because the collation in the system tables (pg_class) is different from
        // that of the information schema, so tables come out of different order in the tables
        // query and the columns query.
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

    fn get_precision(col: &ResultRow) -> Precision {
        let (character_maximum_length, numeric_precision, numeric_scale, time_precision) =
            if matches!(col.get_expect_string("data_type").as_str(), "ARRAY") {
                fn get_single(formatted_type: &str) -> Option<u32> {
                    static SINGLE_REGEX: LazyLock<Regex> =
                        LazyLock::new(|| Regex::new(r".*\(([0-9]*)\).*\[\]$").unwrap());

                    SINGLE_REGEX
                        .captures(formatted_type)
                        .and_then(|cap| cap.get(1))
                        .and_then(|precision| precision.as_str().parse::<u32>().ok())
                }

                fn get_dual(formatted_type: &str) -> (Option<u32>, Option<u32>) {
                    static DUAL_REGEX: LazyLock<Regex> =
                        LazyLock::new(|| Regex::new(r"numeric\(([0-9]*),([0-9]*)\)\[\]$").unwrap());
                    let first = DUAL_REGEX
                        .captures(formatted_type)
                        .and_then(|cap| cap.get(1).and_then(|precision| precision.as_str().parse().ok()));

                    let second = DUAL_REGEX
                        .captures(formatted_type)
                        .and_then(|cap| cap.get(2))
                        .and_then(|precision| precision.as_str().parse::<u32>().ok());

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

    fn get_type_modifiers(col: &ResultRow) -> Option<Vec<i32>> {
        let fdt = col.get_expect_string("formatted_type");

        static TYPE_MODIFIER_REGEX: LazyLock<Regex> = LazyLock::new(|| Regex::new(r".*\(([-0-9, ]+)\)").unwrap());

        TYPE_MODIFIER_REGEX
            .captures(&fdt)
            .and_then(|cap| cap.get(1))
            .map(|modifiers| {
                modifiers
                    .as_str()
                    .split(',')
                    .filter_map(|s| s.trim().parse::<i32>().ok())
                    .collect()
            })
    }

    /// Returns a map from table name to foreign keys.
    async fn get_foreign_keys(
        &self,
        table_ids: &IndexMap<(String, String), TableId>,
        pg_ext: &mut PostgresSchemaExt,
        sql_schema: &mut SqlSchema,
    ) -> DescriberResult<()> {
        let namespaces = &sql_schema.namespaces;

        // The `generate_subscripts` in the inner select is needed because the optimizer is free to reorganize the unnested rows if not explicitly ordered.
        let sql = r#"
            SELECT
                con.oid         AS "con_id",
                att2.attname    AS "child_column",
                cl.relname      AS "parent_table",
                att.attname     AS "parent_column",
                con.confdeltype,
                con.confupdtype,
                rel_ns.nspname  AS "referenced_schema_name",
                conname         AS constraint_name,
                child,
                parent,
                table_name,
                namespace,
                condeferrable,
                condeferred
            FROM (SELECT
                        ns.nspname AS "namespace",
                        unnest(con1.conkey)                AS "parent",
                        unnest(con1.confkey)                AS "child",
                        cl.relname                          AS table_name,
                        ns.nspname                          AS schema_name,
                        generate_subscripts(con1.conkey, 1) AS colidx,
                        con1.oid,
                        con1.confrelid,
                        con1.conrelid,
                        con1.conname,
                        con1.confdeltype,
                        con1.confupdtype,
                        con1.condeferrable                  AS condeferrable,
                        con1.condeferred                    AS condeferred
                FROM pg_class cl
                        join pg_constraint con1 on con1.conrelid = cl.oid
                        join pg_namespace ns on cl.relnamespace = ns.oid
                WHERE
                    ns.nspname = ANY ( $1 )
                    and con1.contype = 'f'
                ORDER BY colidx
                ) con
                    JOIN pg_attribute att on att.attrelid = con.confrelid and att.attnum = con.child
                    JOIN pg_class cl on cl.oid = con.confrelid
                    JOIN pg_attribute att2 on att2.attrelid = con.conrelid and att2.attnum = con.parent
                    JOIN pg_class rel_cl on con.confrelid = rel_cl.oid
                    JOIN pg_namespace rel_ns on rel_cl.relnamespace = rel_ns.oid
            ORDER BY namespace, table_name, constraint_name, con_id, con.colidx;
        "#;

        let mut current_fk: Option<(i64, ForeignKeyId)> = None;

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

        // One foreign key with multiple columns will be represented here as several
        // rows with the same ID.
        let result_set = self.conn.query_raw(sql, &[Value::array(namespaces)]).await?;

        for row in result_set.into_iter() {
            trace!("Got description FK row {:?}", row);
            let id = row.get_expect_i64("con_id");
            let namespace = row.get_expect_string("namespace");
            let table_name = row.get_expect_string("table_name");
            let column_name = row.get_expect_string("child_column");
            let constraint_name = row.get_expect_string("constraint_name");
            let referenced_table = row.get_expect_string("parent_table");
            let referenced_column = row.get_expect_string("parent_column");

            let referenced_schema_name = row.get_expect_string("referenced_schema_name");
            if !sql_schema.namespaces.contains(&referenced_schema_name) {
                return Err(DescriberError::from(DescriberErrorKind::CrossSchemaReference {
                    from: format!("{}.{table_name}", sql_schema.namespaces[0]),
                    to: format!("{referenced_schema_name}.{referenced_table}"),
                    constraint: constraint_name,
                    missing_namespace: referenced_schema_name,
                }));
            }

            let (table_id, column_id, referenced_table_id, referenced_column_id) = if let Some(ids) = get_ids(
                namespace,
                table_name,
                &column_name,
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

            let confdeltype = row
                .get_char("confdeltype")
                .unwrap_or_else(|| row.get_expect_string("confdeltype").chars().next().unwrap());
            let confupdtype = row
                .get_char("confupdtype")
                .unwrap_or_else(|| row.get_expect_string("confupdtype").chars().next().unwrap());

            let on_delete_action = match confdeltype {
                'a' => ForeignKeyAction::NoAction,
                'r' => ForeignKeyAction::Restrict,
                'c' => ForeignKeyAction::Cascade,
                'n' => ForeignKeyAction::SetNull,
                'd' => ForeignKeyAction::SetDefault,
                _ => panic!("unrecognized foreign key action (on delete) '{confdeltype}'"),
            };
            let on_update_action = match confupdtype {
                'a' => ForeignKeyAction::NoAction,
                'r' => ForeignKeyAction::Restrict,
                'c' => ForeignKeyAction::Cascade,
                'n' => ForeignKeyAction::SetNull,
                'd' => ForeignKeyAction::SetDefault,
                _ => panic!("unrecognized foreign key action (on update) '{confupdtype}'"),
            };

            match current_fk {
                Some((current_oid, _)) if current_oid == id => (),
                None | Some(_) => {
                    let fkid = sql_schema.push_foreign_key(
                        Some(constraint_name),
                        [table_id, referenced_table_id],
                        [on_delete_action, on_update_action],
                    );

                    let mut constraint_options = BitFlags::empty();

                    if let Some(true) = row.get_bool("condeferrable") {
                        constraint_options |= ConstraintOption::Deferrable;
                    }

                    if let Some(true) = row.get_bool("condeferred") {
                        constraint_options |= ConstraintOption::Deferred;
                    }

                    pg_ext
                        .constraint_options
                        .insert(Constraint::ForeignKey(fkid), constraint_options);

                    current_fk = Some((id, fkid));
                }
            }

            if let Some((_, fkid)) = current_fk {
                sql_schema.push_foreign_key_column(fkid, [column_id, referenced_column_id]);
            }
        }

        Ok(())
    }

    /// Return the constraints that are not primary keys, foreign keys, or unique keys.
    /// Namely, this returns CHECK and EXCLUDE constraints.
    async fn get_constraints(
        &self,
        table_names: &IndexMap<(String, String), TableId>,
        sql_schema: &mut SqlSchema,
        pg_ext: &mut PostgresSchemaExt,
    ) -> DescriberResult<()> {
        let namespaces = &sql_schema.namespaces;
        let sql = include_str!("postgres/constraints_query.sql");

        let rows = self.conn.query_raw(sql, &[Value::array(namespaces)]).await?;

        for row in rows {
            let namespace = row.get_expect_string("namespace");
            let table_name = row.get_expect_string("table_name");
            let constraint_name = row.get_expect_string("constraint_name");
            let constraint_type = row.get_expect_char("constraint_type");

            let table_id = match table_names.get(&(namespace, table_name)) {
                Some(id) => *id,
                None => continue,
            };

            match constraint_type {
                'c' => {
                    sql_schema.check_constraints.push((table_id, constraint_name));
                }
                'x' => {
                    pg_ext.exclude_constraints.push((table_id, constraint_name));
                }
                _ => (),
            }
        }

        sql_schema.check_constraints.sort_by_key(|(id, _)| *id);
        pg_ext.exclude_constraints.sort_by_key(|(id, _)| *id);

        Ok(())
    }

    async fn get_indices(
        &self,
        table_ids: &IndexMap<(String, String), TableId>,
        pg_ext: &mut PostgresSchemaExt,
        sql_schema: &mut SqlSchema,
    ) -> DescriberResult<()> {
        let namespaces = &sql_schema.namespaces;
        let sql = include_str!("postgres/indexes_query.sql");
        let rows = self.conn.query_raw(sql, &[Value::array(namespaces)]).await?;

        let mut result_rows = Vec::new();
        let mut index_rows = rows.into_iter().peekable();

        loop {
            result_rows.clear();
            group_next_index(&mut result_rows, &mut index_rows);

            if result_rows.is_empty() {
                return Ok(());
            }

            // * Expression Indexes
            if result_rows.iter().any(|row| row.get_string("column_name").is_none()) {
                let row = result_rows.first().unwrap();
                let namespace = row.get_expect_string("namespace");
                let table_name = row.get_expect_string("table_name");
                let table_id: TableId = match table_ids.get(&(namespace, table_name)) {
                    Some(id) => *id,
                    None => continue,
                };

                pg_ext
                    .expression_indexes
                    .push((table_id, row.get_expect_string("index_name")));

                continue;
            }

            index_from_row(result_rows.drain(..), table_ids, sql_schema, pg_ext, self.circumstances);
        }
    }

    async fn get_sequences(&self, sql_schema: &SqlSchema, postgres_ext: &mut PostgresSchemaExt) -> DescriberResult<()> {
        let namespaces = &sql_schema.namespaces;
        // On postgres 9, pg_sequences does not exist, and the information schema view does not
        // contain the cache size.
        let sql = if self.is_cockroach() {
            r#"
              SELECT
                  sequencename AS sequence_name,
                  schemaname AS namespace,
                  start_value,
                  min_value,
                  max_value,
                  increment_by,
                  cycle,
                  cache_size
              FROM pg_sequences
              WHERE schemaname = ANY ( $1 )
              ORDER BY sequence_name
            "#
        } else {
            r#"
              SELECT
                  sequence_name,
                  sequence_schema AS namespace,
                  start_value::INT8,
                  minimum_value::INT8 AS min_value,
                  maximum_value::INT8 AS max_value,
                  increment::INT8 AS increment_by,
                  (CASE cycle_option WHEN 'yes' THEN TRUE ELSE FALSE END) AS cycle,
                  0::INT8 AS cache_size
              FROM information_schema.sequences
              WHERE sequence_schema = ANY ( $1 )
              ORDER BY sequence_name
            "#
        };

        let rows = self.conn.query_raw(sql, &[Value::array(namespaces)]).await?;
        let sequences = rows.into_iter().map(|seq| Sequence {
            namespace_id: sql_schema
                .get_namespace_id(&seq.get_expect_string("namespace"))
                .unwrap(),
            name: seq.get_expect_string("sequence_name"),
            start_value: seq.get_expect_i64("start_value"),
            min_value: seq.get_expect_i64("min_value"),
            max_value: seq.get_expect_i64("max_value"),
            increment_by: seq.get_expect_i64("increment_by"),
            cycle: seq.get_expect_bool("cycle"),
            cache_size: seq.get_expect_i64("cache_size"),
            r#virtual: false,
        });
        postgres_ext.sequences.extend(sequences);

        Ok(())
    }

    async fn get_enums(&self, sql_schema: &mut SqlSchema) -> DescriberResult<()> {
        let namespaces = &sql_schema.namespaces;

        let sql = "
            SELECT
                t.typname AS name,
                e.enumlabel AS value,
                n.nspname AS namespace,
                obj_description(t.oid, 'pg_type') AS description
            FROM pg_type t
            JOIN pg_enum e ON t.oid = e.enumtypid
            JOIN pg_catalog.pg_namespace n ON n.oid = t.typnamespace
            WHERE n.nspname = ANY ( $1 )
            ORDER BY e.enumsortorder";

        let rows = self.conn.query_raw(sql, &[Value::array(namespaces)]).await?;
        let mut enum_values: BTreeMap<(NamespaceId, String, Option<String>), Vec<String>> = BTreeMap::new();

        for row in rows.into_iter() {
            let name = row.get_expect_string("name");
            let value = row.get_expect_string("value");
            let namespace = row.get_expect_string("namespace");
            let description = row.get_string("description");
            let namespace_id = sql_schema.get_namespace_id(&namespace).unwrap();

            let values = enum_values.entry((namespace_id, name, description)).or_default();

            values.push(value);
        }

        for ((namespace_id, enum_name, description), variants) in enum_values {
            let enum_id = sql_schema.push_enum(namespace_id, enum_name, description);

            for variant in variants {
                sql_schema.push_enum_variant(enum_id, variant);
            }
        }

        Ok(())
    }

    async fn get_udts(&self, sql_schema: &mut SqlSchema) -> DescriberResult<()> {
        let namespaces = &sql_schema.namespaces;

        let sql = "
            SELECT
                t.typname AS name,
                n.nspname AS namespace
            FROM pg_type t
            JOIN pg_catalog.pg_namespace n ON n.oid = t.typnamespace
            WHERE n.nspname = ANY ( $1 )
              AND t.typtype = 'b' -- only base types
              AND t.typinput::regproc::text <> 'array_in' -- exclude array types
        ";

        let rows = self.conn.query_raw(sql, &[Value::array(namespaces)]).await?;

        for row in rows.into_iter() {
            let name = row.get_expect_string("name");
            let namespace = row.get_expect_string("namespace");
            let namespace_id = sql_schema.get_namespace_id(&namespace).unwrap();

            sql_schema.push_udt(namespace_id, name, None);
        }

        Ok(())
    }
}

fn group_next_index<T>(result_rows: &mut Vec<ResultRow>, index_rows: &mut Peekable<T>)
where
    T: Iterator<Item = ResultRow>,
{
    let idx_name = match index_rows.peek() {
        Some(idx_name) => idx_name.get_expect_string("index_name"),
        None => return,
    };

    while let Some(row) = index_rows.peek() {
        if row.get_expect_string("index_name") == idx_name {
            result_rows.push(index_rows.next().unwrap());
        } else {
            return;
        }
    }
}

fn index_from_row(
    rows: impl Iterator<Item = ResultRow>,
    table_ids: &IndexMap<(String, String), TableId>,
    sql_schema: &mut SqlSchema,
    pg_ext: &mut PostgresSchemaExt,
    circumstances: BitFlags<Circumstances>,
) {
    let mut current_index = None;

    for row in rows {
        let namespace = row.get_expect_string("namespace");
        let table_name = row.get_expect_string("table_name");
        let table_id: TableId = match table_ids.get(&(namespace, table_name)) {
            Some(id) => *id,
            None => continue,
        };

        let column_name = row.get_expect_string("column_name");

        let column_id = match sql_schema.walk(table_id).column(&column_name) {
            Some(col) => col.id,
            _ => continue,
        };

        let index_name = row.get_expect_string("index_name");
        let is_unique = row.get_expect_bool("is_unique");
        let is_primary_key = row.get_expect_bool("is_primary_key");
        let column_index = row.get_expect_i64("column_index");
        let index_algo = row.get_expect_string("index_algo");
        let predicate = row.get_string("predicate");

        let sort_order = row.get_string("column_order").map(|v| match v.as_ref() {
            "ASC" => SQLSortOrder::Asc,
            "DESC" => SQLSortOrder::Desc,
            misc => panic!("Unexpected sort order `{misc}`, collation should be ASC, DESC or Null"),
        });

        let algorithm = if circumstances.contains(Circumstances::Cockroach) {
            match index_algo.as_str() {
                "inverted" => SqlIndexAlgorithm::Gin,
                _ => SqlIndexAlgorithm::BTree,
            }
        } else {
            match index_algo.as_str() {
                "btree" => SqlIndexAlgorithm::BTree,
                "hash" => SqlIndexAlgorithm::Hash,
                "gist" => SqlIndexAlgorithm::Gist,
                "gin" => SqlIndexAlgorithm::Gin,
                "spgist" => SqlIndexAlgorithm::SpGist,
                "brin" => SqlIndexAlgorithm::Brin,
                other => {
                    tracing::warn!("Unknown index algorithm on {index_name}: {other}");
                    SqlIndexAlgorithm::BTree
                }
            }
        };

        // * Note: Expression Indexes can have 1 & 2
        if column_index == 0 {
            // new index!
            let index_id = if is_primary_key {
                sql_schema.push_primary_key(table_id, index_name)
            } else if is_unique {
                match predicate {
                    Some(pred) => sql_schema.push_partial_unique_constraint(table_id, index_name, pred),
                    None => sql_schema.push_unique_constraint(table_id, index_name),
                }
            } else {
                match predicate {
                    Some(pred) => sql_schema.push_partial_index(table_id, index_name, pred),
                    None => sql_schema.push_index(table_id, index_name),
                }
            };

            if is_primary_key || is_unique {
                let mut constraint_options = BitFlags::empty();

                if let Some(true) = row.get_bool("condeferrable") {
                    constraint_options |= ConstraintOption::Deferrable;
                }

                if let Some(true) = row.get_bool("condeferred") {
                    constraint_options |= ConstraintOption::Deferred;
                }

                pg_ext
                    .constraint_options
                    .insert(Constraint::Index(index_id), constraint_options);
            }

            current_index = Some(index_id);
        }

        let index_id = current_index.unwrap();
        let operator_class = if !matches!(algorithm, SqlIndexAlgorithm::BTree | SqlIndexAlgorithm::Hash) {
            row.get_string("opclass")
                .map(|c| (c, row.get_bool("opcdefault").unwrap_or_default()))
                .map(|(c, is_default)| SQLOperatorClass {
                    kind: SQLOperatorClassKind::from(c.as_str()),
                    is_default,
                })
        } else {
            None
        };

        let index_field_id = sql_schema.push_index_column(IndexColumn {
            index_id,
            column_id,
            sort_order,
            length: None,
        });

        pg_ext.indexes.push((index_id, algorithm));

        // only enable nulls first/last on Postgres
        if !circumstances.contains(Circumstances::Cockroach) && algorithm == SqlIndexAlgorithm::BTree && !is_primary_key
        {
            let nulls_first = row.get_expect_bool("nulls_first");

            let position = if nulls_first {
                IndexNullPosition::First
            } else {
                IndexNullPosition::Last
            };

            pg_ext.index_null_position.insert(index_field_id, position);
        }

        if let Some(opclass) = operator_class {
            pg_ext.opclasses.push((index_field_id, opclass));
        }
    }
}

fn get_column_type_postgresql(row: &ResultRow, schema: &SqlSchema) -> ColumnType {
    let data_type = row.get_expect_string("data_type");
    let full_data_type = row.get_expect_string("full_data_type");
    let is_required = match row.get_expect_string("is_nullable").to_lowercase().as_ref() {
        "no" => true,
        "yes" => false,
        x => panic!("unrecognized is_nullable variant '{x}'"),
    };

    let arity = match matches!(data_type.as_str(), "ARRAY") {
        true => ColumnArity::List,
        false if is_required => ColumnArity::Required,
        false => ColumnArity::Nullable,
    };

    let (family, native_type) = get_column_type_family(&data_type, &full_data_type, row, schema);
    ColumnType {
        full_data_type,
        family,
        arity,
        native_type: native_type.map(NativeTypeInstance::new::<PostgresType>),
    }
}

fn get_column_type_family(
    data_type: &str,
    full_data_type: &str,
    row: &ResultRow,
    schema: &SqlSchema,
) -> (ColumnTypeFamily, Option<PostgresType>) {
    use ColumnTypeFamily::*;

    let precision = SqlSchemaDescriber::get_precision(row);

    let (t, nt) = match full_data_type {
        "int2" | "_int2" => (Int, Some(KnownPostgresType::SmallInt)),
        "int4" | "_int4" => (Int, Some(KnownPostgresType::Integer)),
        "int8" | "_int8" => (BigInt, Some(KnownPostgresType::BigInt)),
        "oid" | "_oid" => (Int, Some(KnownPostgresType::Oid)),
        "float4" | "_float4" => (Float, Some(KnownPostgresType::Real)),
        "float8" | "_float8" => (Float, Some(KnownPostgresType::DoublePrecision)),
        "bool" | "_bool" => (Boolean, Some(KnownPostgresType::Boolean)),
        "text" | "_text" => (String, Some(KnownPostgresType::Text)),
        "citext" | "_citext" => (String, Some(KnownPostgresType::Citext)),
        "varchar" | "_varchar" => (
            String,
            Some(KnownPostgresType::VarChar(precision.character_maximum_length)),
        ),
        "bpchar" | "_bpchar" => (
            String,
            Some(KnownPostgresType::Char(precision.character_maximum_length)),
        ),
        // https://www.cockroachlabs.com/docs/stable/string.html
        "char" | "_char" => (String, Some(KnownPostgresType::Char(None))),
        "date" | "_date" => (DateTime, Some(KnownPostgresType::Date)),
        "bytea" | "_bytea" => (Binary, Some(KnownPostgresType::ByteA)),
        "json" | "_json" => (Json, Some(KnownPostgresType::Json)),
        "jsonb" | "_jsonb" => (Json, Some(KnownPostgresType::JsonB)),
        "uuid" | "_uuid" => (Uuid, Some(KnownPostgresType::Uuid)),
        "xml" | "_xml" => (String, Some(KnownPostgresType::Xml)),
        // bit and varbit should be binary, but are currently mapped to strings.
        "bit" | "_bit" => (String, Some(KnownPostgresType::Bit(precision.character_maximum_length))),
        "varbit" | "_varbit" => (
            String,
            Some(KnownPostgresType::VarBit(precision.character_maximum_length)),
        ),
        "numeric" | "_numeric" => (
            Decimal,
            Some(KnownPostgresType::Decimal(
                match (precision.numeric_precision, precision.numeric_scale) {
                    (None, None) => None,
                    (Some(prec), Some(scale)) => Some((prec, scale)),
                    _ => None,
                },
            )),
        ),
        "money" | "_money" => (Decimal, Some(KnownPostgresType::Money)),
        "time" | "_time" => (DateTime, Some(KnownPostgresType::Time(precision.time_precision))),
        "timetz" | "_timetz" => (DateTime, Some(KnownPostgresType::Timetz(precision.time_precision))),
        "timestamp" | "_timestamp" => (DateTime, Some(KnownPostgresType::Timestamp(precision.time_precision))),
        "timestamptz" | "_timestamptz" => (DateTime, Some(KnownPostgresType::Timestamptz(precision.time_precision))),
        "inet" | "_inet" => (String, Some(KnownPostgresType::Inet)),
        _ => return get_udt_column_type_family(data_type, full_data_type, row, schema),
    };
    (t, nt.map(PostgresType::Known))
}

fn get_udt_column_type_family(
    data_type: &str,
    full_data_type: &str,
    row: &ResultRow,
    schema: &SqlSchema,
) -> (ColumnTypeFamily, Option<PostgresType>) {
    let namespace = row.get_string("type_schema_name");
    if data_type == "USER-DEFINED"
        && let Some(id) = schema.find_enum(full_data_type, namespace.as_deref())
    {
        (ColumnTypeFamily::Enum(id), None)
    } else if data_type == "USER-DEFINED"
        && let Some(id) = schema.find_udt(full_data_type, namespace.as_deref())
    {
        let modifiers = SqlSchemaDescriber::get_type_modifiers(row).unwrap_or_default();
        (
            ColumnTypeFamily::Udt(id),
            Some(PostgresType::Unknown(
                full_data_type.to_owned(),
                modifiers.into_iter().map(|tmod| tmod.to_string()).collect(),
            )),
        )
    } else if data_type == "ARRAY"
        && let Some(id) = schema.find_enum(&full_data_type[1..], namespace.as_deref())
    {
        (ColumnTypeFamily::Enum(id), None)
    } else if data_type == "ARRAY"
        && let Some(id) = schema.find_udt(&full_data_type[1..], namespace.as_deref())
    {
        let modifiers = SqlSchemaDescriber::get_type_modifiers(row).unwrap_or_default();
        (
            ColumnTypeFamily::Udt(id),
            Some(PostgresType::Unknown(
                full_data_type.to_owned(),
                modifiers.into_iter().map(|i| i.to_string()).collect(),
            )),
        )
    } else {
        (ColumnTypeFamily::Unsupported(full_data_type.to_owned()), None)
    }
}

// Separate from get_column_type_postgresql because of native types.
fn get_column_type_cockroachdb(row: &ResultRow, schema: &SqlSchema) -> ColumnType {
    use ColumnTypeFamily::*;
    let data_type = row.get_expect_string("data_type");
    let full_data_type = row.get_expect_string("full_data_type");
    let is_required = match row.get_expect_string("is_nullable").to_lowercase().as_ref() {
        "no" => true,
        "yes" => false,
        x => panic!("unrecognized is_nullable variant '{x}'"),
    };

    let arity = match matches!(data_type.as_str(), "ARRAY") {
        true => ColumnArity::List,
        false if is_required => ColumnArity::Required,
        false => ColumnArity::Nullable,
    };

    let precision = SqlSchemaDescriber::get_precision(row);
    let unsupported_type = || (Unsupported(full_data_type.clone()), None);
    let enum_id: Option<_> = match data_type.as_str() {
        "ARRAY" if full_data_type.starts_with('_') => {
            let namespace = row.get_string("type_schema_name");
            schema.find_enum(&full_data_type[1..], namespace.as_deref())
        }
        _ => {
            let namespace = row.get_string("type_schema_name");
            schema.find_enum(&full_data_type, namespace.as_deref())
        }
    };

    let (family, native_type) = match full_data_type.as_str() {
        _ if data_type == "USER-DEFINED" || data_type == "ARRAY" && enum_id.is_some() => (Enum(enum_id.unwrap()), None),
        "int2" | "_int2" => (Int, Some(CockroachType::Int2)),
        "int4" | "_int4" => (Int, Some(CockroachType::Int4)),
        "int8" | "_int8" => (BigInt, Some(CockroachType::Int8)),
        "float4" | "_float4" => (Float, Some(CockroachType::Float4)),
        "float8" | "_float8" => (Float, Some(CockroachType::Float8)),
        "bool" | "_bool" => (Boolean, Some(CockroachType::Bool)),
        "text" | "_text" => (String, Some(CockroachType::String(precision.character_maximum_length))),
        "varchar" | "_varchar" => (String, Some(CockroachType::String(precision.character_maximum_length))),
        "bpchar" | "_bpchar" => (String, Some(CockroachType::Char(precision.character_maximum_length))),
        // https://www.cockroachlabs.com/docs/stable/string.html
        "char" | "_char" if data_type == "\"char\"" => (String, Some(CockroachType::CatalogSingleChar)),
        "char" | "_char" => (String, Some(CockroachType::Char(precision.character_maximum_length))),
        "date" | "_date" => (DateTime, Some(CockroachType::Date)),
        "bytea" | "_bytea" => (Binary, Some(CockroachType::Bytes)),
        "jsonb" | "_jsonb" => (Json, Some(CockroachType::JsonB)),
        "uuid" | "_uuid" => (Uuid, Some(CockroachType::Uuid)),
        // bit and varbit should be binary, but are currently mapped to strings.
        "bit" | "_bit" => (String, Some(CockroachType::Bit(precision.character_maximum_length))),
        "varbit" | "_varbit" => (String, Some(CockroachType::VarBit(precision.character_maximum_length))),
        "numeric" | "_numeric" => (
            Decimal,
            Some(CockroachType::Decimal(
                match (precision.numeric_precision, precision.numeric_scale) {
                    (None, None) => None,
                    (Some(prec), Some(scale)) => Some((prec, scale)),
                    _ => None,
                },
            )),
        ),
        "pg_lsn" | "_pg_lsn" => unsupported_type(),
        "oid" | "_oid" => (Int, Some(CockroachType::Oid)),
        "time" | "_time" => (DateTime, Some(CockroachType::Time(precision.time_precision))),
        "timetz" | "_timetz" => (DateTime, Some(CockroachType::Timetz(precision.time_precision))),
        "timestamp" | "_timestamp" => (DateTime, Some(CockroachType::Timestamp(precision.time_precision))),
        "timestamptz" | "_timestamptz" => (DateTime, Some(CockroachType::Timestamptz(precision.time_precision))),
        "tsquery" | "_tsquery" => unsupported_type(),
        "tsvector" | "_tsvector" => unsupported_type(),
        "txid_snapshot" | "_txid_snapshot" => unsupported_type(),
        "inet" | "_inet" => (String, Some(CockroachType::Inet)),
        //geometric
        "box" | "_box" => unsupported_type(),
        "circle" | "_circle" => unsupported_type(),
        "line" | "_line" => unsupported_type(),
        "lseg" | "_lseg" => unsupported_type(),
        "path" | "_path" => unsupported_type(),
        "polygon" | "_polygon" => unsupported_type(),
        _ => enum_id.map(|id| (Enum(id), None)).unwrap_or_else(unsupported_type),
    };

    ColumnType {
        full_data_type,
        family,
        arity,
        native_type: native_type.map(NativeTypeInstance::new::<CockroachType>),
    }
}
