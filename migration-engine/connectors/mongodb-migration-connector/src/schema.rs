mod walkers;

pub(crate) use walkers::IndexWalker;

use bson::Document;
use std::collections::BTreeSet;

//
// ==== Storage types ====
//
// These should stay private
//

#[derive(Debug)]
struct Collection {
    name: String,
}

#[derive(Debug)]
struct Index {
    name: String,
    keys: Document,
    is_unique: bool,
    collection_id: CollectionId,
}

//
// === ID types ===
//

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct IndexId(u32);

impl IndexId {
    const MIN: IndexId = IndexId(u32::MIN);
    const MAX: IndexId = IndexId(u32::MAX);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CollectionId(u32);

//
// === The schema ===
//
// Fields should stay private.
//

/// The internal representation of a mongodb database schema.
#[derive(Default, Debug)]
pub(crate) struct MongoSchema {
    /// Collection storage.
    collections: Vec<Collection>,
    /// Index storage.
    indexes: Vec<Index>,
    /// (collection_id, index_id)
    collection_indexes: BTreeSet<(CollectionId, IndexId)>,
}

impl MongoSchema {
    pub(crate) fn push_collection(&mut self, name: String) -> CollectionId {
        self.collections.push(Collection { name });
        CollectionId(self.collections.len() as u32 - 1)
    }

    pub(crate) fn push_index(&mut self, collection_id: CollectionId, name: String, is_unique: bool, keys: Document) {
        let index_id = IndexId(self.indexes.len() as u32);
        self.indexes.push(Index {
            name,
            keys,
            is_unique,
            collection_id,
        });
        self.collection_indexes.insert((collection_id, index_id));
    }
}
