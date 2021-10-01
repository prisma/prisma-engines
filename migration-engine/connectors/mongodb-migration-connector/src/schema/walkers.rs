use super::{Collection, CollectionId, Index, IndexId, MongoSchema};

impl MongoSchema {
    pub(crate) fn walk_collections(&self) -> impl ExactSizeIterator<Item = CollectionWalker<'_>> + '_ {
        (0..self.collections.len()).map(move |idx| self.walk_collection(CollectionId(idx as u32)))
    }

    pub(crate) fn walk_index(&self, id: IndexId) -> IndexWalker<'_> {
        Walker { schema: self, id }
    }

    pub(crate) fn walk_collection(&self, id: CollectionId) -> CollectionWalker<'_> {
        Walker { schema: self, id }
    }
}

#[derive(Clone, Copy)]
pub(crate) struct Walker<'a, T> {
    schema: &'a MongoSchema,
    id: T,
}

impl<'a, T> Walker<'a, T>
where
    T: Copy,
{
    pub(crate) fn schema(&self) -> &'a MongoSchema {
        self.schema
    }

    pub(crate) fn id(&self) -> T {
        self.id
    }
}

pub(crate) type IndexWalker<'a> = Walker<'a, IndexId>;
pub(crate) type CollectionWalker<'a> = Walker<'a, CollectionId>;

impl<'a> IndexWalker<'a> {
    fn get(&self) -> &'a Index {
        &self.schema().indexes[self.id.0 as usize]
    }

    pub(crate) fn collection(self) -> CollectionWalker<'a> {
        self.schema().walk_collection(self.get().collection_id)
    }

    pub(crate) fn is_unique(&self) -> bool {
        self.get().is_unique
    }

    pub(crate) fn name(&self) -> &'a str {
        &self.get().name
    }

    pub(crate) fn keys(self) -> &'a bson::Document {
        &self.get().keys
    }
}

impl<'a> CollectionWalker<'a> {
    fn get(&self) -> &'a Collection {
        &self.schema().collections[self.id.0 as usize]
    }

    pub(crate) fn name(&self) -> &'a str {
        &self.get().name
    }

    pub(crate) fn indexes(&self) -> impl Iterator<Item = IndexWalker<'a>> + 'a {
        let schema = self.schema();
        let id = self.id();

        schema
            .collection_indexes
            .range((id, IndexId::MIN)..(id, IndexId::MAX))
            .map(move |(_col_id, index_id)| schema.walk_index(*index_id))
    }
}
