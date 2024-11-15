use bson::Bson;
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    fmt::{self, Debug},
    ops,
};

use crate::{CollectionWalker, IndexWalker};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
/// An id of a collection in the schema.
pub struct CollectionId(usize);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
/// An id of an index in the schema.
pub struct IndexId(usize);

#[derive(Debug, Clone, Serialize, Deserialize)]
/// All the information we can fetch per collection.
pub struct CollectionData {
    pub(crate) name: String,
    pub(crate) has_schema: bool,
    pub(crate) is_capped: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
/// The type of an index.
pub enum IndexType {
    /// Mapped as `@@index`.
    Normal,
    /// Mapped as `@unique` or `@@unique`.
    Unique,
    /// Mapped as `@@fulltext`.
    Fulltext,
}

impl IndexType {
    /// Is the type defining a full-text index.
    pub fn is_fulltext(self) -> bool {
        matches!(self, Self::Fulltext)
    }
}

impl IndexData {
    /// Is the index defining a full-text index.
    pub fn is_fulltext(&self) -> bool {
        self.r#type.is_fulltext()
    }
}

/// All the information we can scrape per index.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexData {
    /// The name of the index.
    pub name: String,
    /// The type, either fulltext, unique or normal.
    pub r#type: IndexType,
    /// The fields defining the index.
    pub fields: Vec<IndexField>,
    /// The id of a collection this index is part of.
    pub collection_id: CollectionId,
}

/// All the possible information we should scrape out from a MongoDB database.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct MongoSchema {
    collections: Vec<CollectionData>,
    indexes: Vec<IndexData>,
    pub(super) collection_indexes: BTreeMap<CollectionId, Vec<IndexId>>,
}

impl MongoSchema {
    /// Add a collection to the schema.
    pub fn push_collection(&mut self, name: String, has_schema: bool, is_capped: bool) -> CollectionId {
        self.collections.push(CollectionData {
            name,
            has_schema,
            is_capped,
        });
        CollectionId(self.collections.len() - 1)
    }

    /// Adds an index to the schema.
    pub fn push_index(
        &mut self,
        collection_id: CollectionId,
        name: String,
        r#type: IndexType,
        fields: Vec<IndexField>,
    ) -> IndexId {
        self.indexes.push(IndexData {
            name,
            r#type,
            fields,
            collection_id,
        });

        let index_id = IndexId(self.indexes.len() - 1);
        let coll_ind = self.collection_indexes.entry(collection_id).or_default();

        coll_ind.push(index_id);
        index_id
    }

    /// An iterator over all the collections in the schema.
    pub fn walk_collections(&self) -> impl ExactSizeIterator<Item = CollectionWalker<'_>> + '_ {
        self.collections.iter().enumerate().map(|(id, _)| CollectionWalker {
            id: CollectionId(id),
            schema: self,
        })
    }

    /// Walk a collection.
    ///
    /// ## Panics
    ///
    /// If there is no collection with the given id.
    pub fn walk_collection(&self, id: CollectionId) -> CollectionWalker<'_> {
        CollectionWalker { id, schema: self }
    }

    /// Walk an index.
    ///
    /// ## Panics
    ///
    /// If there is no index with the given id.
    pub fn walk_index(&self, id: IndexId) -> IndexWalker<'_> {
        IndexWalker { id, schema: self }
    }

    /// Remove all indexes that are of fulltext type. We basically
    /// need this until the feature is GA.
    ///
    /// Only call this if you do not hold `IndexId`s anywhere.
    pub fn remove_fulltext_indexes(&mut self) {
        self.collection_indexes.clear();

        #[allow(clippy::needless_collect)] // well, mr. clippy, maybe you should read about the borrow checker...
        let indexes: Vec<_> = self.indexes.drain(0..).filter(|i| !i.is_fulltext()).collect();

        for index in indexes.into_iter() {
            let IndexData {
                name,
                r#type,
                fields,
                collection_id,
            } = index;

            // because this here is a mutable reference, so we must collect...
            self.push_index(collection_id, name, r#type, fields);
        }
    }
}

impl ops::Index<CollectionId> for MongoSchema {
    type Output = CollectionData;

    fn index(&self, index: CollectionId) -> &Self::Output {
        &self.collections[index.0]
    }
}

impl ops::Index<IndexId> for MongoSchema {
    type Output = IndexData;

    fn index(&self, index: IndexId) -> &Self::Output {
        &self.indexes[index.0]
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
/// A field that is part of an index.
pub struct IndexField {
    /// The name of the field.
    pub name: String,
    /// Defines the property of the field.
    pub property: IndexFieldProperty,
}

impl IndexField {
    /// The name of the field.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Is the field part of a full-text index.
    pub fn is_text(&self) -> bool {
        matches!(self.property, IndexFieldProperty::Text)
    }
}

impl fmt::Display for IndexField {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "\"{}\":{}", self.name, self.property)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
/// Defines the property in the field that is part of an index.
pub enum IndexFieldProperty {
    /// Defines a full-text index.
    Text,
    /// Sorted ascending.
    Ascending,
    /// Sorted descending.
    Descending,
}

impl IndexFieldProperty {
    /// If the property is descending.
    pub fn is_descending(self) -> bool {
        matches!(self, Self::Descending)
    }
}

impl fmt::Display for IndexFieldProperty {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            IndexFieldProperty::Text => f.write_str("\"text\""),
            IndexFieldProperty::Ascending => f.write_str("1"),
            IndexFieldProperty::Descending => f.write_str("-1"),
        }
    }
}

impl From<IndexFieldProperty> for Bson {
    fn from(property: IndexFieldProperty) -> Self {
        match property {
            IndexFieldProperty::Text => Bson::String(String::from("text")),
            IndexFieldProperty::Ascending => Bson::Int32(1),
            IndexFieldProperty::Descending => Bson::Int32(-1),
        }
    }
}
