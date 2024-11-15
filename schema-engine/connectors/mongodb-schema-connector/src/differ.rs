use crate::migration::{MongoDbMigration, MongoDbMigrationStep};
use mongodb_schema_describer::{CollectionId, IndexField, IndexId, IndexWalker, MongoSchema};
use std::collections::BTreeMap;

pub(crate) fn diff(previous: Box<MongoSchema>, next: Box<MongoSchema>) -> MongoDbMigration {
    let mut steps = Vec::new();
    let db = DifferDatabase::new(previous.as_ref(), next.as_ref());

    for collection_id in db.created_collections() {
        steps.push(MongoDbMigrationStep::CreateCollection(collection_id));

        for index in next.walk_collection(collection_id).indexes() {
            steps.push(MongoDbMigrationStep::CreateIndex(index.id()));
        }
    }

    for index in db.created_indexes() {
        steps.push(MongoDbMigrationStep::CreateIndex(index.id()))
    }

    for index in db.dropped_indexes() {
        steps.push(MongoDbMigrationStep::DropIndex(index.id()))
    }

    steps.sort(); // important: this determines the order of execution

    MongoDbMigration { previous, next, steps }
}

struct DifferDatabase<'a> {
    collections: BTreeMap<&'a str, (Option<CollectionId>, Option<CollectionId>)>,
    #[allow(clippy::type_complexity)] // respectfully disagree
    indexes: BTreeMap<(CollectionId, CollectionId, &'a str), (Option<IndexId>, Option<IndexId>)>,
    previous: &'a MongoSchema,
    next: &'a MongoSchema,
}

impl<'a> DifferDatabase<'a> {
    fn new(previous: &'a MongoSchema, next: &'a MongoSchema) -> Self {
        let mut collections = BTreeMap::new();
        let mut indexes = BTreeMap::new();

        for collection in previous.walk_collections() {
            collections.insert(collection.name(), (Some(collection.id()), None));
        }

        for collection in next.walk_collections() {
            let entry = collections.entry(collection.name()).or_default();
            entry.1 = Some(collection.id());

            if let Some(previous_collection_id) = entry.0 {
                for index in previous.walk_collection(previous_collection_id).indexes() {
                    indexes.insert(
                        (previous_collection_id, collection.id(), index.name()),
                        (Some(index.id()), None),
                    );
                }

                for index in collection.indexes() {
                    let entry = indexes
                        .entry((previous_collection_id, collection.id(), index.name()))
                        .or_default();
                    entry.1 = Some(index.id());
                }
            }
        }

        DifferDatabase {
            collections,
            indexes,
            previous,
            next,
        }
    }

    fn created_collections(&self) -> impl Iterator<Item = CollectionId> + '_ {
        self.collections
            .values()
            .filter(|(previous, _)| previous.is_none())
            .filter_map(|(_, next)| next.as_ref().cloned())
    }

    /// Iterate created indexes over all _collection pairs_ (collections that exist in both previous and next schema).
    fn created_indexes(&self) -> impl Iterator<Item = IndexWalker<'a>> + '_ {
        self.all_indexes()
            .filter_map(|(previous, next)| match (previous, next) {
                (Some(previous), Some(next)) if indexes_are_different(previous, next) => Some(next),
                (None, next) => next,
                (Some(_), _) => None,
            })
    }

    /// Iterate created indexes over all _collection pairs_ (collections that exist in both previous and next schema).
    fn dropped_indexes(&self) -> impl Iterator<Item = IndexWalker<'a>> + '_ {
        self.all_indexes()
            .filter_map(|(previous, next)| match (previous, next) {
                (Some(previous), Some(next)) if indexes_are_different(previous, next) => Some(previous),
                (previous, None) => previous,
                (_, Some(_)) => None,
            })
    }

    fn all_indexes(&self) -> impl Iterator<Item = (Option<IndexWalker<'a>>, Option<IndexWalker<'a>>)> + '_ {
        self.indexes.values().map(move |(previous_id, next_id)| {
            let previous = previous_id.map(|previous_id| self.previous.walk_index(previous_id));
            let next = next_id.map(|next_id| self.next.walk_index(next_id));
            (previous, next)
        })
    }
}

fn indexes_are_different(previous: IndexWalker<'_>, next: IndexWalker<'_>) -> bool {
    // sigh
    if previous.is_fulltext() && next.is_fulltext() {
        let is_fts = |v: &&IndexField| v.is_text();

        let previous_heads: Vec<_> = previous.fields().take_while(|field| !is_fts(field)).collect();
        let next_heads: Vec<_> = next.fields().take_while(|field| !is_fts(field)).collect();

        // the middles will come in a wrong order from the database. We must be able to compare
        // them as equal no matter the order, because the generated index is not per field, but to
        // an abstract `$text` field that just holds data from the text columns.
        let mut previous_middles: Vec<_> = previous.fields().take_while(is_fts).collect();
        previous_middles.sort_by(|left, right| left.name().cmp(right.name()));

        let mut next_middles: Vec<_> = next.fields().take_while(is_fts).collect();
        next_middles.sort_by(|left, right| left.name().cmp(right.name()));

        let previous_tails: Vec<_> = previous
            .fields()
            .skip_while(|field| !is_fts(field))
            .skip_while(is_fts)
            .collect();

        let next_tails: Vec<_> = next
            .fields()
            .skip_while(|field| !is_fts(field))
            .skip_while(is_fts)
            .collect();

        previous_heads != next_heads || previous_middles != next_middles || previous_tails != next_tails
    } else {
        // We don't compare names here because we assume it has been done earlier.
        previous.r#type() != next.r#type() || !keys_match(previous.fields(), next.fields())
    }
}

fn keys_match<'schema, T>(previous: T, next: T) -> bool
where
    T: ExactSizeIterator<Item = &'schema IndexField> + 'schema,
{
    previous.len() == next.len() && previous.zip(next).all(|(prev, next)| prev == next)
}
