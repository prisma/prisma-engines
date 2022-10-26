use std::fmt;

use crate::{
    datamodel::attributes::BlockAttribute,
    value::{Array, Constant, Function, Text, Value},
};

use super::IndexFieldInput;

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
    pub fn name(&mut self, name: &'a str) {
        self.0.push_param(("name", Text(name)));
    }

    /// The constraint name in the database, defined as the `map`
    /// argument inside the attribute.
    pub fn map(&mut self, map: &'a str) {
        self.0.push_param(("map", Text(map)));
    }

    /// Defines the `clustered` argument inside the attribute.
    pub fn clustered(&mut self, clustered: bool) {
        self.0.push_param(("clustered", Constant::new_no_validate(clustered)));
    }

    /// Defines the `type` argument inside the attribute.
    pub fn index_type(&mut self, index_type: &'a str) {
        self.0.push_param(("type", Constant::new_no_validate(index_type)));
    }

    fn new(index_type: &'static str, fields: impl Iterator<Item = IndexFieldInput<'a>>) -> Self {
        let mut inner = Function::new(index_type);

        let fields: Vec<_> = fields.map(Function::from).map(Value::Function).collect();
        inner.push_param(Value::Array(Array::from(fields)));

        Self(BlockAttribute(inner))
    }
}

impl<'a> fmt::Display for IndexDefinition<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

/// Index type definition.
#[derive(Debug, Clone, Copy)]
pub enum IndexOps<'a> {
    /// Managed and known by Prisma. Renders as-is as a constant.
    ///
    /// ```ignore
    /// @@index([field(ops: Int2BloomOps)], type: Brin)
    /// //                  ^^^^^^^^^^^^ like this
    /// ```
    Managed(&'a str),
    /// A type we don't handle yet. Renders as raw.
    ///
    /// ```ignore
    /// @@index([field(ops: raw("tsvector_ops"))], type: Gist)
    /// //                  ^^^^^^^^^^^^^^^^^^^ like this
    /// ```
    Raw(Text<&'a str>),
}

impl<'a> fmt::Display for IndexOps<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Managed(s) => f.write_str(s),
            Self::Raw(s) => {
                write!(f, "raw({s})")
            }
        }
    }
}
