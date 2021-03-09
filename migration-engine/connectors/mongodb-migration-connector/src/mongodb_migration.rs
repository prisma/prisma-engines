use migration_connector::DatabaseMigrationMarker;

#[derive(Debug)]
pub struct MongoDbMigration {
    pub(crate) steps: Vec<MongoDbMigrationStep>,
}

impl DatabaseMigrationMarker for MongoDbMigration {
    const FILE_EXTENSION: &'static str = "mongo";

    fn is_empty(&self) -> bool {
        self.steps.is_empty()
    }
}

#[derive(Debug)]
pub(crate) enum MongoDbMigrationStep {
    CreateCollection(String),
}
