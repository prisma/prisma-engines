#[derive(Debug)]
pub struct MongoDbMigration {
    pub(crate) steps: Vec<MongoDbMigrationStep>,
}

impl MongoDbMigration {
    pub(crate) fn summary(&self) -> String {
        let mut out = String::with_capacity(self.steps.len() * 10);

        for step in &self.steps {
            match step {
                MongoDbMigrationStep::CreateCollection(collection) => {
                    out.push_str("- Added collection `");
                    out.push_str(collection);
                    out.push_str("`\n");
                }
            }
        }

        out
    }
}

#[derive(Debug)]
pub(crate) enum MongoDbMigrationStep {
    CreateCollection(String),
}
