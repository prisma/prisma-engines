mod field;
mod id;

use std::{borrow::Cow, fmt};

use crate::{
    datamodel::attributes::BlockAttribute,
    value::{Array, Constant, Function, Text, Value},
};

pub use field::{IndexFieldInput, UniqueFieldAttribute};
pub use id::{IdDefinition, IdFieldDefinition};

/// Defines an index in a model block.
#[derive(Debug)]
pub struct IndexDefinition<'a>(BlockAttribute<'a>);

impl<'a> IndexDefinition<'a> {
    /// A normal index, defined as `@@index`.
    pub fn index(fields: impl Iterator<Item = IndexFieldInput<'a>>) -> Self {
        Self::new("index", fields)
    }

    /// A unique constraint, defined as `@@unique`.
    pub fn unique(fields: impl Iterator<Item = IndexFieldInput<'a>>) -> Self {
        Self::new("unique", fields)
    }

    /// A fulltext index, defined as `@@fulltext`.
    pub fn fulltext(fields: impl Iterator<Item = IndexFieldInput<'a>>) -> Self {
        Self::new("fulltext", fields)
    }

    /// The client name of the index, defined as the `name` argument
    /// inside the attribute.
    pub fn name(&mut self, name: impl Into<Cow<'a, str>>) {
        self.0.push_param(("name", Text::new(name)));
    }

    /// The constraint name in the database, defined as the `map`
    /// argument inside the attribute.
    pub fn map(&mut self, map: impl Into<Cow<'a, str>>) {
        self.0.push_param(("map", Text::new(map)));
    }

    /// Defines the `clustered` argument inside the attribute.
    pub fn clustered(&mut self, clustered: bool) {
        self.0.push_param(("clustered", Constant::new_no_validate(clustered)));
    }

    /// Defines the `type` argument inside the attribute.
    pub fn index_type(&mut self, index_type: impl Into<Cow<'a, str>>) {
        self.0
            .push_param(("type", Constant::new_no_validate(index_type.into())));
    }

    /// Defines the `where` argument for partial indexes.
    pub fn where_clause(&mut self, where_clause: impl Into<Cow<'a, str>>) {
        self.0.push_param(("where", Text::new(where_clause)));
    }

    fn new(index_type: &'static str, fields: impl Iterator<Item = IndexFieldInput<'a>>) -> Self {
        let mut inner = Function::new(index_type);

        let fields: Vec<_> = fields.map(Function::from).map(Value::Function).collect();
        inner.push_param(Value::Array(Array::from(fields)));

        Self(BlockAttribute(inner))
    }
}

impl fmt::Display for IndexDefinition<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

#[derive(Debug, Clone)]
pub enum InnerOps<'a> {
    Managed(Cow<'a, str>),
    Raw(Text<Cow<'a, str>>),
}

/// Index field operator definition.
#[derive(Debug, Clone)]
pub struct IndexOps<'a>(InnerOps<'a>);

impl<'a> IndexOps<'a> {
    /// Managed and known by Prisma. Renders as-is as a constant.
    ///
    /// ```ignore
    /// @@index([field(ops: Int2BloomOps)], type: Brin)
    /// //                  ^^^^^^^^^^^^ like this
    /// ```
    pub fn managed(ops: impl Into<Cow<'a, str>>) -> Self {
        Self(InnerOps::Managed(ops.into()))
    }

    /// A type we don't handle yet. Renders as raw.
    ///
    /// ```ignore
    /// @@index([field(ops: raw("tsvector_ops"))], type: Gist)
    /// //                  ^^^^^^^^^^^^^^^^^^^ like this
    /// ```
    pub fn raw(ops: impl Into<Cow<'a, str>>) -> Self {
        Self(InnerOps::Raw(Text(ops.into())))
    }
}

impl fmt::Display for IndexOps<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.0 {
            InnerOps::Managed(s) => f.write_str(s),
            InnerOps::Raw(s) => {
                write!(f, "raw({s})")
            }
        }
    }
}
