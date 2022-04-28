//! Database description. This crate is used heavily in the introspection and migration engines.
#![allow(clippy::trivial_regex)] // this is allowed, because we want to do CoW replaces and these regexes will grow.
#![allow(clippy::match_bool)] // we respectfully disagree that it makes the code less readable.

pub mod mssql;
pub mod mysql;
pub mod postgres;
pub mod sqlite;
pub mod walkers;

pub(crate) mod common;

mod connector_data;
mod error;
mod getters;
mod ids;
mod parsers;

pub use self::{
    error::{DescriberError, DescriberErrorKind, DescriberResult},
    ids::*,
    postgres::{SQLOperatorClass, SQLOperatorClassKind},
};

use once_cell::sync::Lazy;
use prisma_value::PrismaValue;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::{
    any::Any,
    fmt::{self, Debug},
};
use walkers::{EnumWalker, TableWalker, UserDefinedTypeWalker, ViewWalker};

/// A database description connector.
#[async_trait::async_trait]
pub trait SqlSchemaDescriberBackend: Send + Sync {
    /// List the database's schemas.
    async fn list_databases(&self) -> DescriberResult<Vec<String>>;

    /// Get the databases metadata.
    async fn get_metadata(&self, schema: &str) -> DescriberResult<SqlMetadata>;

    /// Describe a database schema.
    async fn describe(&self, schema: &str) -> DescriberResult<SqlSchema>;

    /// Get the database version.
    async fn version(&self, schema: &str) -> DescriberResult<Option<String>>;
}

pub struct SqlMetadata {
    pub table_count: usize,
    pub size_in_bytes: usize,
}

/// The result of describing a database schema.
#[derive(Serialize, Deserialize, Debug, PartialEq, Default)]
pub struct SqlSchema {
    /// The schema's tables.
    pub tables: Vec<Table>,
    /// The schema's enums.
    pub enums: Vec<Enum>,
    /// The schema's sequences, unique to Postgres.
    sequences: Vec<Sequence>,
    /// The schema's views,
    views: Vec<View>,
    /// The stored procedures.
    procedures: Vec<Procedure>,
    /// The user-defined types procedures.
    user_defined_types: Vec<UserDefinedType>,
    /// Connector-specific data
    connector_data: connector_data::ConnectorData,
}

impl SqlSchema {
    /// Extract connector-specific constructs. The type parameter must be the right one.
    pub fn downcast_connector_data<T: 'static>(&self) -> &T {
        self.connector_data.data.downcast_ref().unwrap()
    }

    /// Extract connector-specific constructs. The type parameter must be the right one.
    pub fn downcast_connector_data_mut<T: 'static>(&mut self) -> &mut T {
        self.connector_data.data.downcast_mut().unwrap()
    }

    pub fn set_connector_data(&mut self, data: Box<dyn Any + Send + Sync>) {
        self.connector_data.data = data;
    }

    /// Get a table.
    pub fn get_table(&self, name: &str) -> Option<&Table> {
        self.tables.iter().find(|x| x.name == name)
    }

    /// Get a view.
    pub fn get_view(&self, name: &str) -> Option<&View> {
        self.views.iter().find(|v| v.name == name)
    }

    /// Get an enum.
    pub fn get_enum(&self, name: &str) -> Option<&Enum> {
        self.enums.iter().find(|x| x.name == name)
    }

    /// Get a procedure.
    pub fn get_procedure(&self, name: &str) -> Option<&Procedure> {
        self.procedures.iter().find(|x| x.name == name)
    }

    pub fn get_user_defined_type(&self, name: &str) -> Option<&UserDefinedType> {
        self.user_defined_types.iter().find(|x| x.name == name)
    }

    /// Is this schema empty?
    pub fn is_empty(&self) -> bool {
        matches!(
            self,
            SqlSchema {
                tables,
                enums,
                sequences,
                views,
                procedures,
                user_defined_types,
                ..
            } if tables.is_empty() && enums.is_empty() && sequences.is_empty() && views.is_empty() && procedures.is_empty() && user_defined_types.is_empty()
        )
    }

    pub fn iter_tables(&self) -> impl Iterator<Item = (TableId, &Table)> {
        self.tables
            .iter()
            .enumerate()
            .map(|(table_index, table)| (TableId(table_index as u32), table))
    }

    pub fn iter_tables_mut(&mut self) -> impl Iterator<Item = (TableId, &mut Table)> {
        self.tables
            .iter_mut()
            .enumerate()
            .map(|(table_index, table)| (TableId(table_index as u32), table))
    }

    pub fn table(&self, name: &str) -> core::result::Result<&Table, String> {
        match self.tables.iter().find(|t| t.name == name) {
            Some(t) => Ok(t),
            None => Err(name.to_string()),
        }
    }

    pub fn table_bang(&self, name: &str) -> &Table {
        self.table(name).unwrap()
    }

    /// Get a sequence.
    pub fn get_sequence(&self, name: &str) -> Option<&Sequence> {
        self.sequences.iter().find(|x| x.name == name)
    }

    pub fn empty() -> SqlSchema {
        SqlSchema::default()
    }

    pub fn table_walkers(&self) -> impl Iterator<Item = TableWalker<'_>> {
        (0..self.tables.len()).map(move |table_index| TableWalker::new(self, TableId(table_index as u32)))
    }

    pub fn view_walkers(&self) -> impl Iterator<Item = ViewWalker<'_>> {
        (0..self.views.len()).map(move |view_index| ViewWalker::new(self, view_index))
    }

    pub fn udt_walkers(&self) -> impl Iterator<Item = UserDefinedTypeWalker<'_>> {
        (0..self.user_defined_types.len()).map(move |udt_index| UserDefinedTypeWalker::new(self, udt_index))
    }

    pub fn enum_walkers(&self) -> impl Iterator<Item = EnumWalker<'_>> {
        (0..self.enums.len()).map(move |enum_index| EnumWalker {
            schema: self,
            enum_index,
        })
    }
}

/// A table found in a schema.
#[derive(Serialize, Deserialize, PartialEq, Debug, Default)]
pub struct Table {
    /// The table's name.
    pub name: String,
    /// The table's columns.
    pub columns: Vec<Column>,
    /// The table's indices.
    pub indices: Vec<Index>,
    /// The table's primary key, if there is one.
    pub primary_key: Option<PrimaryKey>,
    /// The table's foreign keys.
    pub foreign_keys: Vec<ForeignKey>,
}

impl Table {
    pub fn column_bang(&self, name: &str) -> &Column {
        self.column(name)
            .unwrap_or_else(|| panic!("Column {} not found in Table {}", name, self.name))
    }

    pub fn column<'a>(&'a self, name: &str) -> Option<&'a Column> {
        self.columns.iter().find(|c| c.name == name)
    }

    pub fn has_column(&self, name: &str) -> bool {
        self.column(name).is_some()
    }

    pub fn is_part_of_foreign_key(&self, column: &str) -> bool {
        self.foreign_key_for_column(column).is_some()
    }

    pub fn foreign_key_for_column(&self, column: &str) -> Option<&ForeignKey> {
        self.foreign_keys
            .iter()
            .find(|fk| fk.columns.contains(&column.to_string()))
    }

    pub fn is_part_of_primary_key(&self, column: &str) -> bool {
        match &self.primary_key {
            Some(pk) => pk.columns.iter().any(|c| c.name() == column),
            None => false,
        }
    }

    pub fn primary_key_columns(&self) -> impl Iterator<Item = &PrimaryKeyColumn> + '_ {
        match &self.primary_key {
            Some(pk) => pk.columns.iter(),
            None => [].iter(),
        }
    }

    pub fn is_column_unique(&self, column_name: &str) -> bool {
        self.indices.iter().any(|index| {
            index.is_unique() && index.columns.len() == 1 && index.columns.iter().any(|c| c.name() == column_name)
        })
    }

    pub fn is_column_primary_key(&self, column_name: &str) -> bool {
        match &self.primary_key {
            None => false,
            Some(key) => key.is_single_primary_key(column_name),
        }
    }
}

/// The type of an index.
#[derive(Serialize, Deserialize, PartialEq, Debug, Clone, Copy)]
pub enum IndexType {
    /// Unique type.
    Unique,
    /// Normal type.
    Normal,
    /// Fulltext type.
    Fulltext,
}

impl IndexType {
    pub fn is_unique(&self) -> bool {
        matches!(self, IndexType::Unique)
    }
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone, Copy)]
pub enum SQLIndexAlgorithm {
    BTree,
    Hash,
    Gist,
    Gin,
    SpGist,
    Brin,
}

impl Default for SQLIndexAlgorithm {
    fn default() -> Self {
        Self::BTree
    }
}

impl AsRef<str> for SQLIndexAlgorithm {
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

impl fmt::Display for SQLIndexAlgorithm {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_ref())
    }
}

/// The sort order of an index.
#[derive(Serialize, Deserialize, PartialEq, Debug, Copy, Clone)]
pub enum SQLSortOrder {
    Asc,
    Desc,
}

impl Default for SQLSortOrder {
    fn default() -> Self {
        Self::Asc
    }
}

impl AsRef<str> for SQLSortOrder {
    fn as_ref(&self) -> &str {
        match self {
            SQLSortOrder::Asc => "ASC",
            SQLSortOrder::Desc => "DESC",
        }
    }
}

impl fmt::Display for SQLSortOrder {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_ref())
    }
}

#[derive(Default, Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct IndexColumn {
    pub name: String,
    pub sort_order: Option<SQLSortOrder>,
    pub length: Option<u32>,
}

impl IndexColumn {
    pub fn new(name: impl ToString) -> Self {
        Self {
            name: name.to_string(),
            ..Default::default()
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn set_sort_order(&mut self, sort_order: SQLSortOrder) {
        self.sort_order = Some(sort_order);
    }
}

/// An index of a table.
#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct Index {
    /// Index name.
    pub name: String,
    /// Index columns.
    pub columns: Vec<IndexColumn>,
    /// Type of index.
    pub tpe: IndexType,
    /// BTree or Hash
    pub algorithm: Option<SQLIndexAlgorithm>,
}

impl Index {
    pub fn is_unique(&self) -> bool {
        self.tpe == IndexType::Unique
    }

    pub fn is_fulltext(&self) -> bool {
        self.tpe == IndexType::Fulltext
    }

    pub fn column_names(&self) -> impl ExactSizeIterator<Item = &str> + '_ {
        self.columns.iter().map(|c| c.name())
    }
}

/// A stored procedure (like, the function inside your database).
#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct Procedure {
    /// Procedure name.
    pub name: String,
    /// The definition of the procedure.
    pub definition: Option<String>,
}

/// A user-defined type. Can map to another type, or be declared as assembly.
#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct UserDefinedType {
    /// Type name
    pub name: String,
    /// Type mapping
    pub definition: Option<String>,
}

#[derive(Default, Serialize, Deserialize, Debug, Clone)]
pub struct PrimaryKeyColumn {
    pub name: String,
    pub length: Option<u32>,
    pub sort_order: Option<SQLSortOrder>,
}

impl PartialEq for PrimaryKeyColumn {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name && self.length == other.length && self.sort_order() == other.sort_order()
    }
}

impl PrimaryKeyColumn {
    pub fn new(name: impl ToString) -> Self {
        Self {
            name: name.to_string(),
            ..Default::default()
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn set_sort_order(&mut self, sort_order: SQLSortOrder) {
        self.sort_order = Some(sort_order);
    }

    pub fn sort_order(&self) -> SQLSortOrder {
        self.sort_order.unwrap_or(SQLSortOrder::Asc)
    }
}

/// The primary key of a table.
#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct PrimaryKey {
    /// Columns.
    pub columns: Vec<PrimaryKeyColumn>,
    /// The sequence optionally seeding this primary key.
    pub sequence: Option<Sequence>,
    /// The name of the primary key constraint, when available.
    pub constraint_name: Option<String>,
}

impl PrimaryKey {
    pub fn is_single_primary_key(&self, column: &str) -> bool {
        self.columns.len() == 1 && self.columns.iter().any(|col| col.name() == column)
    }

    pub fn column_names(&self) -> impl ExactSizeIterator<Item = &str> + '_ {
        self.columns.iter().map(|c| c.name())
    }
}

/// A column of a table.
#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub struct Column {
    /// Column name.
    pub name: String,
    /// Column type.
    pub tpe: ColumnType,
    /// Column default.
    pub default: Option<DefaultValue>,
    /// Is the column auto-incrementing?
    pub auto_increment: bool,
}

impl Column {
    pub fn is_required(&self) -> bool {
        self.tpe.arity == ColumnArity::Required
    }
}

/// The type of a column.
#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub struct ColumnType {
    /// The full SQL data type, the sql string necessary to recreate the column, drawn directly from the db, used when there is no native type.
    pub full_data_type: String,
    /// The family of the raw type.
    pub family: ColumnTypeFamily,
    /// The arity of the column.
    pub arity: ColumnArity,
    /// The Native type of the column.
    pub native_type: Option<serde_json::Value>,
}

impl ColumnType {
    pub fn pure(family: ColumnTypeFamily, arity: ColumnArity) -> Self {
        ColumnType {
            full_data_type: "".to_string(),
            family,
            arity,
            native_type: None,
        }
    }

    pub fn with_full_data_type(family: ColumnTypeFamily, arity: ColumnArity, full_data_type: String) -> Self {
        ColumnType {
            full_data_type,
            family,
            arity,
            native_type: None,
        }
    }
}

/// Enumeration of column type families.
#[derive(Serialize, Deserialize, PartialEq, Debug)]
// TODO: this name feels weird.
pub enum ColumnTypeFamily {
    /// Integer types.
    Int,
    /// BigInt types.
    BigInt,
    /// Floating point types.
    Float,
    /// Decimal Types.
    Decimal,
    /// Boolean types.
    Boolean,
    /// String types.
    String,
    /// DateTime types.
    DateTime,
    /// Binary types.
    Binary,
    /// JSON types.
    Json,
    /// UUID types.
    Uuid,
    ///Enum
    Enum(String),
    /// Unsupported
    Unsupported(String),
}

impl ColumnTypeFamily {
    pub fn as_enum(&self) -> Option<&str> {
        match self {
            ColumnTypeFamily::Enum(name) => Some(name),
            _ => None,
        }
    }

    pub fn is_bigint(&self) -> bool {
        matches!(self, ColumnTypeFamily::BigInt)
    }

    pub fn is_boolean(&self) -> bool {
        matches!(self, ColumnTypeFamily::Boolean)
    }

    pub fn is_datetime(&self) -> bool {
        matches!(self, ColumnTypeFamily::DateTime)
    }

    pub fn is_enum(&self) -> bool {
        matches!(self, ColumnTypeFamily::Enum(_))
    }

    pub fn is_int(&self) -> bool {
        matches!(self, ColumnTypeFamily::Int)
    }

    pub fn is_json(&self) -> bool {
        matches!(self, ColumnTypeFamily::Json)
    }

    pub fn is_string(&self) -> bool {
        matches!(self, ColumnTypeFamily::String)
    }
}

/// A column's arity.
#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub enum ColumnArity {
    /// Required column.
    Required,
    /// Nullable column.
    Nullable,
    /// List type column.
    List,
}

impl ColumnArity {
    /// The arity is ColumnArity::List.
    pub fn is_list(&self) -> bool {
        matches!(self, ColumnArity::List)
    }

    /// The arity is ColumnArity::Nullable.
    pub fn is_nullable(&self) -> bool {
        matches!(self, ColumnArity::Nullable)
    }

    /// The arity is ColumnArity::Required.
    pub fn is_required(&self) -> bool {
        matches!(self, ColumnArity::Required)
    }
}

/// Foreign key action types (for ON DELETE|ON UPDATE).
#[derive(Serialize, Deserialize, PartialEq, Debug, Clone, Copy)]
pub enum ForeignKeyAction {
    /// Produce an error indicating that the deletion or update would create a foreign key
    /// constraint violation. If the constraint is deferred, this error will be produced at
    /// constraint check time if there still exist any referencing rows. This is the default action.
    NoAction,
    /// Produce an error indicating that the deletion or update would create a foreign key
    /// constraint violation. This is the same as NO ACTION except that the check is not deferrable.
    Restrict,
    /// Delete any rows referencing the deleted row, or update the values of the referencing
    /// column(s) to the new values of the referenced columns, respectively.
    Cascade,
    /// Set the referencing column(s) to null.
    SetNull,
    /// Set the referencing column(s) to their default values. (There must be a row in the
    /// referenced table matching the default values, if they are not null, or the operation
    /// will fail).
    SetDefault,
}

impl ForeignKeyAction {
    pub fn is_cascade(&self) -> bool {
        matches!(self, ForeignKeyAction::Cascade)
    }
}

/// A foreign key.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ForeignKey {
    /// The database name of the foreign key constraint, when available.
    pub constraint_name: Option<String>,
    /// Column names.
    pub columns: Vec<String>,
    /// Referenced table.
    pub referenced_table: String,
    /// Referenced columns.
    pub referenced_columns: Vec<String>,
    /// Action on deletion.
    pub on_delete_action: ForeignKeyAction,
    /// Action on update.
    pub on_update_action: ForeignKeyAction,
}

impl PartialEq for ForeignKey {
    fn eq(&self, other: &Self) -> bool {
        self.columns == other.columns
            && self.referenced_table == other.referenced_table
            && self.referenced_columns == other.referenced_columns
    }
}

/// A SQL enum.
#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct Enum {
    /// Enum name.
    pub name: String,
    /// Possible enum values.
    pub values: Vec<String>,
}

/// A SQL sequence.
#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct Sequence {
    /// Sequence name.
    pub name: String,
}

/// An SQL view.
#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct View {
    /// Name of the view.
    pub name: String,
    /// The SQL definition of the view.
    pub definition: Option<String>,
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct DefaultValue {
    kind: DefaultKind,
    constraint_name: Option<String>,
}

/// A DefaultValue
#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub enum DefaultKind {
    /// A constant value, parsed as String
    Value(PrismaValue),
    /// An expression generating a current timestamp.
    Now,
    /// An expression generating a sequence.
    Sequence(String),
    /// An unrecognized Default Value
    DbGenerated(String),
}

impl DefaultValue {
    pub fn db_generated(val: impl Into<String>) -> Self {
        Self::new(DefaultKind::DbGenerated(val.into()))
    }

    pub fn now() -> Self {
        Self::new(DefaultKind::Now)
    }

    pub fn value(val: impl Into<PrismaValue>) -> Self {
        Self::new(DefaultKind::Value(val.into()))
    }

    pub fn sequence(val: impl ToString) -> Self {
        Self::new(DefaultKind::Sequence(val.to_string()))
    }

    pub fn new(kind: DefaultKind) -> Self {
        Self {
            kind,
            constraint_name: None,
        }
    }

    pub fn kind(&self) -> &DefaultKind {
        &self.kind
    }

    pub fn into_kind(self) -> DefaultKind {
        self.kind
    }

    pub fn set_constraint_name(&mut self, name: impl ToString) {
        self.constraint_name = Some(name.to_string())
    }

    pub fn constraint_name(&self) -> Option<&str> {
        self.constraint_name.as_deref()
    }

    pub fn as_value(&self) -> Option<&PrismaValue> {
        match self.kind {
            DefaultKind::Value(ref v) => Some(v),
            _ => None,
        }
    }

    pub fn is_value(&self) -> bool {
        matches!(self.kind, DefaultKind::Value(_))
    }

    pub fn is_now(&self) -> bool {
        matches!(self.kind, DefaultKind::Now)
    }

    pub fn is_sequence(&self) -> bool {
        matches!(self.kind, DefaultKind::Sequence(_))
    }

    pub fn is_db_generated(&self) -> bool {
        matches!(self.kind, DefaultKind::DbGenerated(_))
    }

    pub fn with_constraint_name(mut self, constraint_name: Option<String>) -> Self {
        self.constraint_name = constraint_name;
        self
    }
}

pub fn unquote_string(val: &str) -> String {
    val.trim_start_matches('\'')
        .trim_end_matches('\'')
        .trim_start_matches('\\')
        .trim_start_matches('"')
        .trim_end_matches('"')
        .trim_end_matches('\\')
        .into()
}

#[derive(Debug)]
struct Precision {
    character_maximum_length: Option<u32>,
    numeric_precision: Option<u32>,
    numeric_scale: Option<u32>,
    time_precision: Option<u32>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unquoting_works() {
        let quoted_str = "'abc $$ def'".to_string();

        assert_eq!(unquote_string(&quoted_str), "abc $$ def");

        assert_eq!(unquote_string("heh "), "heh ");
    }
}
