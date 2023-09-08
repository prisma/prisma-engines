use crate::{CollectionData, CollectionId, IndexData, IndexField, IndexId, IndexType, MongoSchema};

#[derive(Clone, Copy)]
/// A collection/table in the database.
pub struct CollectionWalker<'schema> {
    pub(crate) id: CollectionId,
    pub(crate) schema: &'schema MongoSchema,
}

impl<'schema> CollectionWalker<'schema> {
    fn get(self) -> &'schema CollectionData {
        &self.schema[self.id]
    }

    /// The id of the collection in the schema.
    pub fn id(self) -> CollectionId {
        self.id
    }

    /// The name of the collection.
    pub fn name(self) -> &'schema str {
        &self.get().name
    }

    /// Iterator over all the indexes in the collection.
    pub fn indexes(self) -> impl ExactSizeIterator<Item = IndexWalker<'schema>> + 'schema {
        let create_walker = move |id: &IndexId| IndexWalker {
            id: *id,
            schema: self.schema,
        };

        match self.schema.collection_indexes.get(&self.id) {
            Some(indexes) => indexes.iter().map(create_walker),
            None => [].iter().map(create_walker),
        }
    }

    /// Checks whether JSONSchema is defined.
    pub fn has_schema(self) -> bool {
        self.get().has_schema
    }

    /// Checks whether the collection is capped.
    pub fn is_capped(self) -> bool {
        self.get().is_capped
    }
}

#[derive(Clone, Copy)]
/// An index in the database.
pub struct IndexWalker<'schema> {
    pub(crate) id: IndexId,
    pub(crate) schema: &'schema MongoSchema,
}

impl<'schema> IndexWalker<'schema> {
    fn get(self) -> &'schema IndexData {
        &self.schema[self.id]
    }

    /// The id of the index in the schema.
    pub fn id(self) -> IndexId {
        self.id
    }

    /// The collection the index is part of.
    pub fn collection(self) -> CollectionWalker<'schema> {
        CollectionWalker {
            id: self.get().collection_id,
            schema: self.schema,
        }
    }

    /// The type of the index.
    pub fn r#type(self) -> IndexType {
        self.get().r#type
    }

    /// The name of the index.
    pub fn name(self) -> &'schema str {
        &self.get().name
    }

    /// True if the index is a full-text index.
    pub fn is_fulltext(self) -> bool {
        matches!(self.r#type(), IndexType::Fulltext)
    }

    /// True if the index is a unique constraint.
    pub fn is_unique(self) -> bool {
        matches!(self.r#type(), IndexType::Unique)
    }

    /// An iterator over the fields defining the index.
    pub fn fields(self) -> impl ExactSizeIterator<Item = &'schema IndexField> + 'schema {
        self.get().fields.iter()
    }
}
