//! Functions and types for conveniently traversing and querying a SqlSchema.

#![deny(missing_docs)]

mod column;
mod default;
mod r#enum;
mod foreign_key;
mod index;
mod namespace;
mod table;
mod user_defined_type;
mod view;

use std::ops::Range;

pub use column::{ColumnWalker, IndexColumnWalker, TableColumnWalker, ViewColumnWalker};
pub use default::{DefaultValueWalker, TableDefaultValueWalker, ViewDefaultValueWalker};
pub use r#enum::{EnumVariantWalker, EnumWalker};
pub use foreign_key::ForeignKeyWalker;
pub use index::IndexWalker;
pub use namespace::NamespaceWalker;
pub use table::TableWalker;
pub use user_defined_type::UserDefinedTypeWalker;
pub use view::ViewWalker;

use crate::SqlSchema;

/// A generic reference to a schema item. It holds a reference to the schema so it can offer a
/// convenient API based on the Id type.
#[derive(Clone, Copy)]
pub struct Walker<'a, Id> {
    /// The identifier.
    pub id: Id,
    /// The schema for which the identifier is valid.
    pub schema: &'a SqlSchema,
}

impl<I: std::fmt::Debug> std::fmt::Debug for Walker<'_, I> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct(std::any::type_name::<Self>())
            .field("id", &self.id)
            .finish()
    }
}

impl<'a, Id> Walker<'a, Id> {
    /// Jump to the item identified by `other_id`.
    pub fn walk<I>(self, other_id: I) -> Walker<'a, I> {
        self.schema.walk(other_id)
    }
}

/// For a slice sorted by a key K, return the contiguous range of items matching the key.
fn range_for_key<I, K>(slice: &[I], key: K, extract: fn(&I) -> K) -> Range<usize>
where
    K: Copy + Ord + PartialOrd + PartialEq,
{
    let seed = slice.binary_search_by_key(&key, extract).unwrap_or(0);
    let mut iter = slice[..seed].iter();
    let start = match iter.rposition(|i| extract(i) != key) {
        None => 0,
        Some(other) => other + 1,
    };
    let mut iter = slice[seed..].iter();
    let end = seed + iter.position(|i| extract(i) != key).unwrap_or(slice.len() - seed);
    start..end
}
