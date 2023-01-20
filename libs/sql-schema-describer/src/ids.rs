use serde::{Deserialize, Serialize};

/// The identifier for a table in a SqlSchema.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct TableId(pub(crate) u32);

/// The identifier for an enum in a SqlSchema.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct EnumId(pub(crate) u32);

/// The identifier for an enum variant in a SqlSchema.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EnumVariantId(pub(crate) u32);

/// The identifier for a column in a SqlSchema.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ColumnId(pub(crate) u32);

/// The identifier for an Index in a SqlSchema.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct IndexId(pub(crate) u32);

/// The identifier for a column indexed by a specific index in a SqlSchema.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct IndexColumnId(pub(crate) u32);

/// The identifier for a foreign key in the schema.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ForeignKeyId(pub(crate) u32);

/// The identifier for a namespace in the schema.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Default)]
pub struct NamespaceId(pub(crate) u32);

/// The identifier for a user defined type in the database.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct UdtId(pub(crate) u32);

/// The identifier for a view in the database.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ViewId(pub(crate) u32);

/// The identifier for a default value in the database.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Serialize, Deserialize, Ord, Hash)]
pub struct DefaultValueId(pub(crate) u32);
