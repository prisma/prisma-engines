use crate::schema::*;

#[derive(Debug)]
pub(crate) struct MongoDbMigration {
    pub(crate) previous: MongoSchema,
    pub(crate) next: MongoSchema,
    pub(crate) steps: Vec<MongoDbMigrationStep>,
}

impl MongoDbMigration {
    pub(crate) fn summary(&self) -> String {
        let mut out = String::with_capacity(self.steps.len() * 10);

        for step in &self.steps {
            match step {
                MongoDbMigrationStep::CreateCollection(collection_id) => {
                    out.push_str("[+] Collection `");
                    out.push_str(self.next.walk_collection(*collection_id).name());
                    out.push_str("`\n");
                }
                MongoDbMigrationStep::CreateIndex(index_id) => {
                    let index = self.next.walk_index(*index_id);
                    out.push_str("[+] ");
                    out.push_str(if index.is_unique() { "Unique index `" } else { "Index `" });
                    out.push_str(index.name());
                    out.push_str("` on (");
                    out.push_str(&serde_json::to_string(index.keys()).unwrap());
                    out.push_str(")\n");
                }
                MongoDbMigrationStep::DropIndex(index_id) => {
                    let index = self.previous.walk_index(*index_id);
                    out.push_str("[-] ");
                    out.push_str(if index.is_unique() { "Unique index `" } else { "Index `" });
                    out.push_str(index.name());
                    out.push_str("`\n");
                }
            }
        }

        out
    }
}

/// The internal representation of a mongodb migration. The order of variants matters, it is used
/// for sorting and determines the order in which steps will be applied.
#[derive(Debug, PartialOrd, Ord, PartialEq, Eq)]
pub enum MongoDbMigrationStep {
    CreateCollection(CollectionId),
    DropIndex(IndexId),
    CreateIndex(IndexId),
}
