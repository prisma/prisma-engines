//! Database description. This crate is used heavily in the schema engine.

#![deny(rust_2018_idioms, unsafe_code)]
#![allow(clippy::derive_partial_eq_without_eq)]

pub mod mssql;
pub mod mysql;
pub mod postgres;
pub mod sqlite;
pub mod walkers;

mod cloneable_any;
mod connector_data;
mod error;
mod getters;
mod ids;
mod parsers;

use crate::cloneable_any::CloneableAny;

pub use self::{
    error::{DescriberError, DescriberErrorKind, DescriberResult},
    ids::*,
    walkers::*,
};
pub use either::Either;
use indexmap::IndexSet;
pub use prisma_value::PrismaValue;

use enumflags2::{BitFlag, BitFlags};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::fmt::{self, Debug};

/// A database description connector.
#[async_trait::async_trait]
pub trait SqlSchemaDescriberBackend: Send + Sync {
    /// Describe a database schema.
    async fn describe(&self, schemas: &[&str]) -> DescriberResult<SqlSchema>;

    /// Get the database version.
    async fn version(&self) -> DescriberResult<Option<String>>;
}

/// The result of describing a database schema.
#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct SqlSchema {
    /// Namespaces (schemas)
    namespaces: IndexSet<String>,
    /// The schema's tables.
    tables: Vec<Table>,
    /// The schema's enums.
    enums: Vec<Enum>,
    enum_variants: Vec<EnumVariant>,
    /// The schema's columns that are in tables.
    table_columns: Vec<(TableId, Column)>,
    /// All foreign keys.
    foreign_keys: Vec<ForeignKey>,
    /// All table column default values.
    table_default_values: Vec<(TableColumnId, DefaultValue)>,
    /// All view column default values.
    view_default_values: Vec<(ViewColumnId, DefaultValue)>,
    /// Constrained and referenced columns of foreign keys.
    foreign_key_columns: Vec<ForeignKeyColumn>,
    /// All indexes and unique constraints.
    indexes: Vec<Index>,
    /// All columns of indexes.
    index_columns: Vec<IndexColumn>,
    /// Check constraints for every table.
    check_constraints: Vec<(TableId, String)>,
    /// The schema's views,
    views: Vec<View>,
    /// The schema's columns that are in views.
    view_columns: Vec<(ViewId, Column)>,
    /// The stored procedures.
    procedures: Vec<Procedure>,
    /// The user-defined types procedures.
    user_defined_types: Vec<UserDefinedType>,
    /// Connector-specific data
    connector_data: connector_data::ConnectorData,
    /// The default runtime namespace, if one is set.
    runtime_namespace: Option<String>,
}

impl SqlSchema {
    /// Extract connector-specific constructs. The type parameter must be the right one.
    #[track_caller]
    pub fn downcast_connector_data<T: 'static>(&self) -> &T {
        self.connector_data.data.as_ref().unwrap().downcast_ref().unwrap()
    }

    /// The id of the next column
    pub fn next_table_column_id(&self) -> TableColumnId {
        TableColumnId(self.table_columns.len() as u32)
    }

    /// The id of the next column
    pub fn next_view_column_id(&self) -> ViewColumnId {
        ViewColumnId(self.view_columns.len() as u32)
    }

    /// The given enum is used in tables.
    pub fn enum_used_in_tables(&self, id: EnumId) -> bool {
        self.table_columns
            .iter()
            .any(|col| col.1.tpe.family == ColumnTypeFamily::Enum(id))
    }

    /// Extract connector-specific constructs mutably. The type parameter must be the right one.
    #[track_caller]
    pub fn downcast_connector_data_mut<T: 'static>(&mut self) -> &mut T {
        self.connector_data.data.as_mut().unwrap().downcast_mut().unwrap()
    }

    /// Remove all namespaces from the schema.
    pub fn clear_namespaces(&mut self) {
        self.namespaces.clear();
    }

    /// Insert connector-specific data into the schema. This will replace existing connector data.
    pub fn set_connector_data(&mut self, data: Box<dyn CloneableAny>) {
        self.connector_data.data = Some(data);
    }

    /// Get a view.
    pub fn get_view(&self, name: &str) -> Option<&View> {
        self.views.iter().find(|v| v.name == name)
    }

    /// Try to find an enum by name.
    pub fn find_enum(&self, name: &str, namespace: Option<&str>) -> Option<EnumId> {
        let ns_id = namespace.and_then(|ns| self.get_namespace(ns));

        self.enums
            .iter()
            .position(|e| e.name == name && ns_id.map(|id| id == e.namespace_id).unwrap_or(true))
            .map(|i| EnumId(i as u32))
    }

    /// Try to find a UDT by name.
    pub fn find_udt(&self, name: &str, namespace: Option<&str>) -> Option<UdtId> {
        let ns_id = namespace.and_then(|ns| self.get_namespace(ns));

        self.user_defined_types
            .iter()
            .position(|u| u.name == name && ns_id.map(|id| id == u.namespace_id).unwrap_or(true))
            .map(|i| UdtId(i as u32))
    }

    fn get_namespace(&self, name: &str) -> Option<NamespaceId> {
        self.namespaces
            .iter()
            .position(|ns| ns == name)
            .map(|i| NamespaceId(i as u32))
    }

    /// Try to find a table by name.
    pub fn find_table(&self, name: &str, namespace: Option<&str>) -> Option<TableId> {
        let ns_id = namespace.and_then(|ns| self.get_namespace(ns));

        self.tables
            .iter()
            .position(|t| t.name == name && ns_id.map(|id| id == t.namespace_id).unwrap_or(true))
            .map(|i| TableId(i as u32))
    }

    /// Try to find a table by name.
    pub fn find_view(&self, name: &str, namespace: Option<&str>) -> Option<ViewId> {
        let ns_id = namespace.and_then(|ns| self.get_namespace(ns));

        self.views
            .iter()
            .position(|t| t.name == name && ns_id.map(|id| id == t.namespace_id).unwrap_or(true))
            .map(|i| ViewId(i as u32))
    }

    /// Get a procedure.
    pub fn get_procedure(&self, name: &str) -> Option<&Procedure> {
        self.procedures.iter().find(|x| x.name == name)
    }

    /// Get a user defined type by name.
    pub fn get_user_defined_type(&self, name: &str) -> Option<&UserDefinedType> {
        self.user_defined_types.iter().find(|x| x.name == name)
    }

    /// Find a namespace by name.
    pub fn get_namespace_id(&self, name: &str) -> Option<NamespaceId> {
        self.namespaces.get_index_of(name).map(|pos| NamespaceId(pos as u32))
    }

    /// The total number of indexes in the schema.
    pub fn indexes_count(&self) -> usize {
        self.indexes.len()
    }

    /// Make all fulltext indexes non-fulltext, for the preview feature's purpose.
    pub fn make_fulltext_indexes_normal(&mut self) {
        for idx in self.indexes.iter_mut() {
            if matches!(idx.tpe, IndexType::Fulltext) {
                idx.tpe = IndexType::Normal;
            }
        }
    }

    /// Clear all index predicates so the differ ignores partial indexes when the feature is disabled.
    pub fn clear_index_predicates(&mut self) {
        for idx in self.indexes.iter_mut() {
            idx.predicate = None;
        }
    }

    /// Add a table column to the schema.
    pub fn push_table_column(&mut self, table_id: TableId, column: Column) -> TableColumnId {
        let id = TableColumnId(self.table_columns.len() as u32);
        self.table_columns.push((table_id, column));
        id
    }

    /// Add a view column to the schema.
    pub fn push_view_column(&mut self, view_id: ViewId, column: Column) -> ViewColumnId {
        let id = ViewColumnId(self.view_columns.len() as u32);
        self.view_columns.push((view_id, column));
        id
    }

    /// Add an enum to the schema.
    pub fn push_enum(&mut self, namespace_id: NamespaceId, enum_name: String, description: Option<String>) -> EnumId {
        let id = EnumId(self.enums.len() as u32);

        self.enums.push(Enum {
            namespace_id,
            name: enum_name,
            description,
        });

        id
    }

    /// Add a variant to an enum.
    pub fn push_enum_variant(&mut self, enum_id: EnumId, variant_name: String) -> EnumVariantId {
        let id = EnumVariantId(self.enum_variants.len() as u32);
        self.enum_variants.push(EnumVariant { enum_id, variant_name });
        id
    }

    /// Add a UDT to the schema.
    pub fn push_udt(&mut self, namespace_id: NamespaceId, name: String, definition: Option<String>) -> UdtId {
        let id = UdtId(self.user_defined_types.len() as u32);

        self.user_defined_types.push(UserDefinedType {
            namespace_id,
            name,
            definition,
        });

        id
    }

    /// Add an index of a certain type to the schema.
    pub fn push_index_of_type(&mut self, table_id: TableId, index_name: String, tpe: IndexType) -> IndexId {
        self.push_index_with_predicate(table_id, index_name, tpe, None)
    }

    /// Add an index of a certain type with an optional predicate (for partial indexes).
    pub fn push_index_with_predicate(
        &mut self,
        table_id: TableId,
        index_name: String,
        tpe: IndexType,
        predicate: Option<String>,
    ) -> IndexId {
        let id = IndexId(self.indexes.len() as u32);
        self.indexes.push(Index {
            table_id,
            index_name,
            tpe,
            predicate,
        });
        id
    }

    /// Add a fulltext index to the schema.
    pub fn push_fulltext_index(&mut self, table_id: TableId, index_name: String) -> IndexId {
        self.push_index_of_type(table_id, index_name, IndexType::Fulltext)
    }

    /// Add an index to the schema.
    pub fn push_index(&mut self, table_id: TableId, index_name: String) -> IndexId {
        self.push_index_of_type(table_id, index_name, IndexType::Normal)
    }

    /// Add a partial index to the schema (with a WHERE clause predicate).
    pub fn push_partial_index(&mut self, table_id: TableId, index_name: String, predicate: String) -> IndexId {
        self.push_index_with_predicate(table_id, index_name, IndexType::Normal, Some(predicate))
    }

    /// Add a partial unique constraint/index to the schema (with a WHERE clause predicate).
    pub fn push_partial_unique_constraint(
        &mut self,
        table_id: TableId,
        index_name: String,
        predicate: String,
    ) -> IndexId {
        self.push_index_with_predicate(table_id, index_name, IndexType::Unique, Some(predicate))
    }

    /// Add table default value to the schema.
    pub fn push_table_default_value(&mut self, column_id: TableColumnId, value: DefaultValue) -> TableDefaultValueId {
        let id = TableDefaultValueId(self.table_default_values.len() as u32);
        self.table_default_values.push((column_id, value));
        id
    }

    /// Add table default value to the schema.
    pub fn push_view_default_value(&mut self, column_id: ViewColumnId, value: DefaultValue) -> ViewDefaultValueId {
        let id = ViewDefaultValueId(self.view_default_values.len() as u32);
        self.view_default_values.push((column_id, value));
        id
    }

    /// Add a primary key to the schema.
    pub fn push_primary_key(&mut self, table_id: TableId, index_name: String) -> IndexId {
        self.push_index_of_type(table_id, index_name, IndexType::PrimaryKey)
    }

    /// Add a unique constraint/index to the schema.
    pub fn push_unique_constraint(&mut self, table_id: TableId, index_name: String) -> IndexId {
        self.push_index_of_type(table_id, index_name, IndexType::Unique)
    }

    pub fn push_index_column(&mut self, column: IndexColumn) -> IndexColumnId {
        let id = IndexColumnId(self.index_columns.len() as u32);
        self.index_columns.push(column);
        id
    }

    pub fn push_foreign_key(
        &mut self,
        constraint_name: Option<String>,
        [constrained_table, referenced_table]: [TableId; 2],
        [on_delete_action, on_update_action]: [ForeignKeyAction; 2],
    ) -> ForeignKeyId {
        let id = ForeignKeyId(self.foreign_keys.len() as u32);
        self.foreign_keys.push(ForeignKey {
            constrained_table,
            constraint_name,
            referenced_table,
            on_delete_action,
            on_update_action,
        });
        id
    }

    pub fn push_foreign_key_column(
        &mut self,
        foreign_key_id: ForeignKeyId,
        [constrained_column, referenced_column]: [TableColumnId; 2],
    ) {
        self.foreign_key_columns.push(ForeignKeyColumn {
            foreign_key_id,
            constrained_column,
            referenced_column,
        });
    }

    pub fn push_namespace(&mut self, name: String) -> NamespaceId {
        let (id, _) = self.namespaces.insert_full(name);
        NamespaceId(id as u32)
    }

    pub fn push_table(&mut self, name: String, namespace_id: NamespaceId, description: Option<String>) -> TableId {
        let id = TableId(self.tables.len() as u32);

        self.tables.push(Table {
            namespace_id,
            name,
            properties: TableProperties::empty(),
            description,
        });

        id
    }

    pub fn push_view(
        &mut self,
        name: String,
        namespace_id: NamespaceId,
        definition: Option<String>,
        description: Option<String>,
    ) -> ViewId {
        let id = ViewId(self.views.len() as u32);

        self.views.push(View {
            namespace_id,
            name,
            definition,
            description,
        });

        id
    }

    pub fn push_table_with_properties(
        &mut self,
        name: String,
        namespace_id: NamespaceId,
        properties: BitFlags<TableProperties>,
        description: Option<String>,
    ) -> TableId {
        let id = TableId(self.tables.len() as u32);

        self.tables.push(Table {
            namespace_id,
            name,
            properties,
            description,
        });

        id
    }

    pub fn namespaces_count(&self) -> usize {
        self.namespaces.len()
    }

    pub fn namespace_walker<'a>(&'a self, name: &str) -> Option<NamespaceWalker<'a>> {
        let namespace_idx = self.namespaces.iter().position(|ns| ns == name)?;
        Some(self.walk(NamespaceId(namespace_idx as u32)))
    }

    pub fn tables_count(&self) -> usize {
        self.tables.len()
    }

    pub fn views_count(&self) -> usize {
        self.views.len()
    }

    pub fn table_walker<'a>(&'a self, name: &str) -> Option<TableWalker<'a>> {
        let table_idx = self.tables.iter().position(|table| table.name == name)?;
        Some(self.walk(TableId(table_idx as u32)))
    }

    pub fn table_walker_ns<'a>(&'a self, namespace: &str, name: &str) -> Option<TableWalker<'a>> {
        let namespace_idx = self.namespace_walker(namespace)?.id;

        let table_idx = self
            .tables
            .iter()
            .position(|table| table.name == name && table.namespace_id == namespace_idx)?;

        Some(self.walk(TableId(table_idx as u32)))
    }

    pub fn view_walker<'a>(&'a self, name: &str) -> Option<ViewWalker<'a>> {
        let view_idx = self.views.iter().position(|view| view.name == name)?;
        Some(self.walk(ViewId(view_idx as u32)))
    }

    pub fn view_walker_ns<'a>(&'a self, namespace: &str, name: &str) -> Option<ViewWalker<'a>> {
        let namespace_idx = self.namespace_walker(namespace)?.id;

        let view_idx = self
            .views
            .iter()
            .position(|view| view.name == name && view.namespace_id == namespace_idx)?;

        Some(self.walk(ViewId(view_idx as u32)))
    }

    pub fn table_walkers(&self) -> impl ExactSizeIterator<Item = TableWalker<'_>> {
        (0..self.tables.len()).map(move |table_index| self.walk(TableId(table_index as u32)))
    }

    pub fn view_walkers(&self) -> impl ExactSizeIterator<Item = ViewWalker<'_>> {
        (0..self.views.len()).map(move |view_index| self.walk(ViewId(view_index as u32)))
    }

    pub fn udt_walkers(&self) -> impl Iterator<Item = UserDefinedTypeWalker<'_>> {
        (0..self.user_defined_types.len()).map(move |udt_index| self.walk(UdtId(udt_index as u32)))
    }

    pub fn enum_walkers(&self) -> impl ExactSizeIterator<Item = EnumWalker<'_>> {
        (0..self.enums.len()).map(move |enum_index| self.walk(EnumId(enum_index as u32)))
    }

    pub fn walk_foreign_keys(&self) -> impl Iterator<Item = ForeignKeyWalker<'_>> {
        (0..self.foreign_keys.len()).map(move |fk_idx| ForeignKeyWalker {
            schema: self,
            id: ForeignKeyId(fk_idx as u32),
        })
    }

    /// Traverse a schema item by id.
    pub fn walk<I>(&self, id: I) -> Walker<'_, I> {
        Walker { id, schema: self }
    }

    /// Traverse all the table columns in the schema.
    pub fn walk_table_columns(&self) -> impl Iterator<Item = TableColumnWalker<'_>> {
        (0..self.table_columns.len()).map(|idx| self.walk(TableColumnId(idx as u32)))
    }

    /// Traverse all the table columns in the schema.
    pub fn walk_view_columns(&self) -> impl Iterator<Item = ViewColumnWalker<'_>> {
        (0..self.view_columns.len()).map(|idx| self.walk(ViewColumnId(idx as u32)))
    }

    /// Traverse all namespaces in the catalog.
    pub fn walk_namespaces(&self) -> impl ExactSizeIterator<Item = NamespaceWalker<'_>> {
        (0..self.namespaces.len()).map(|idx| self.walk(NamespaceId(idx as u32)))
    }

    /// No tables or enums in the catalog.
    pub fn is_empty(&self) -> bool {
        self.tables.is_empty() && self.enums.is_empty()
    }

    /// Get the default namespace, if any.
    pub fn default_namespace(&self) -> Option<&str> {
        self.runtime_namespace.as_deref()
    }

    /// Set the default namespace.
    pub fn set_default_namespace(&mut self, namespace: String) {
        self.runtime_namespace = Some(namespace);
    }
}

#[enumflags2::bitflags]
#[repr(u8)]
#[derive(Clone, Copy, Debug)]
pub enum TableProperties {
    IsPartition,
    HasSubclass,
    HasRowLevelSecurity,
}

/// A table found in a schema.
#[derive(Serialize, Deserialize, PartialEq, Debug, Default, Clone)]
pub struct Table {
    namespace_id: NamespaceId,
    name: String,
    properties: BitFlags<TableProperties>,
    description: Option<String>,
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
    /// The table's primary key
    PrimaryKey,
}

/// The sort order of an index.
#[derive(Serialize, Deserialize, PartialEq, Debug, Copy, Clone, Default)]
pub enum SQLSortOrder {
    #[default]
    Asc,
    Desc,
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

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct IndexColumn {
    pub index_id: IndexId,
    pub column_id: TableColumnId,
    pub sort_order: Option<SQLSortOrder>,
    pub length: Option<u32>,
}

/// An index on a table.
#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
struct Index {
    table_id: TableId,
    index_name: String,
    tpe: IndexType,
    predicate: Option<String>,
}

/// A stored procedure (like, the function inside your database).
#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct Procedure {
    ///Namespace of the procedure
    namespace_id: NamespaceId,
    /// Procedure name.
    pub name: String,
    /// The definition of the procedure.
    pub definition: Option<String>,
}

/// A user-defined type. Can map to another type, or be declared as assembly.
#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct UserDefinedType {
    /// Namespace of the UDT.
    pub namespace_id: NamespaceId,
    /// Type name
    pub name: String,
    /// Type mapping
    pub definition: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Column {
    /// Column name.
    pub name: String,
    /// Column type.
    pub tpe: ColumnType,
    /// Is the column auto-incrementing?
    pub auto_increment: bool,
    /// The comment in the database
    pub description: Option<String>,
}

/// The type of a column.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ColumnType {
    /// The full SQL data type, the sql string necessary to recreate the column, drawn directly from the db, used when there is no native type.
    pub full_data_type: String,
    /// The family of the raw type.
    pub family: ColumnTypeFamily,
    /// The arity of the column.
    pub arity: ColumnArity,
    /// The Native type of the column.
    #[serde(skip)]
    pub native_type: Option<psl::datamodel_connector::NativeTypeInstance>,
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
#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
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
    /// Enum
    Enum(EnumId),
    /// User-defined type
    Udt(UdtId),
    /// Unsupported
    Unsupported(String),
}

impl ColumnTypeFamily {
    pub fn as_enum(&self) -> Option<EnumId> {
        match self {
            ColumnTypeFamily::Enum(id) => Some(*id),
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

    pub fn is_unsupported(&self) -> bool {
        matches!(self, ColumnTypeFamily::Unsupported(_))
    }
}

/// A column's arity.
#[derive(Serialize, Deserialize, PartialEq, Debug, Clone, Copy)]
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

#[derive(Serialize, Deserialize, Debug, Clone)]
struct ForeignKey {
    /// The table the foreign key is defined on.
    constrained_table: TableId,
    /// Referenced table.
    referenced_table: TableId,
    /// The foreign key constraint name, when available.
    constraint_name: Option<String>,
    on_delete_action: ForeignKeyAction,
    on_update_action: ForeignKeyAction,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct ForeignKeyColumn {
    foreign_key_id: ForeignKeyId,
    constrained_column: TableColumnId,
    referenced_column: TableColumnId,
}

/// A SQL enum.
#[derive(Serialize, Deserialize, Debug, Clone)]
struct Enum {
    /// The namespace the enum type belongs to, if applicable.
    namespace_id: NamespaceId,
    name: String,
    description: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct EnumVariant {
    enum_id: EnumId,
    variant_name: String,
}

/// An SQL view.
#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct View {
    /// Namespace of the view
    namespace_id: NamespaceId,
    /// Name of the view.
    pub name: String,
    /// The SQL definition of the view.
    pub definition: Option<String>,
    /// The comment in the database
    pub description: Option<String>,
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
    /// A unique row ID,
    UniqueRowid,
    /// An unrecognized Default Value
    DbGenerated(Option<String>),
}

impl DefaultValue {
    pub fn db_generated<S: Into<String>>(val: impl Into<Option<S>>) -> Self {
        Self::new(DefaultKind::DbGenerated(val.into().map(Into::into)))
    }

    pub fn constraint_name(&self) -> Option<&str> {
        self.constraint_name.as_deref()
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

    pub fn kind(&self) -> &DefaultKind {
        &self.kind
    }

    pub fn new(kind: DefaultKind) -> Self {
        Self {
            kind,
            constraint_name: None,
        }
    }

    pub fn set_constraint_name(&mut self, name: impl ToString) {
        self.constraint_name = Some(name.to_string())
    }

    pub(crate) fn as_value(&self) -> Option<&PrismaValue> {
        match self.kind {
            DefaultKind::Value(ref v) => Some(v),
            _ => None,
        }
    }

    #[cfg(test)]
    pub(crate) fn as_sequence(&self) -> Option<&str> {
        match self.kind {
            DefaultKind::Sequence(ref name) => Some(name),
            _ => None,
        }
    }

    #[cfg(test)]
    pub(crate) fn is_db_generated(&self) -> bool {
        matches!(self.kind, DefaultKind::DbGenerated(_))
    }

    pub fn unique_rowid() -> Self {
        Self::new(DefaultKind::UniqueRowid)
    }

    pub fn with_constraint_name(mut self, constraint_name: Option<String>) -> Self {
        self.constraint_name = constraint_name;
        self
    }

    /// If the default value is the deprecated `dbgenerated()`
    /// variant.
    pub fn is_empty_dbgenerated(&self) -> bool {
        matches!(self.kind, DefaultKind::DbGenerated(None))
    }
}

fn unquote_string(val: &str) -> String {
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
