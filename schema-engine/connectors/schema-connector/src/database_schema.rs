/// A database schema. Part of the MigrationConnector API.
pub struct DatabaseSchema(Box<dyn std::any::Any + Send + Sync>);

impl DatabaseSchema {
    /// Type-erase a migration.
    pub fn new<T: 'static + Send + Sync>(migration: T) -> Self {
        DatabaseSchema(Box::new(migration))
    }

    /// Should never be used in the core, only in connectors that know what they put there.
    pub fn downcast<T: 'static>(self) -> Box<T> {
        self.0.downcast().unwrap()
    }

    /// Should never be used in the core, only in connectors that know what they put there.
    pub fn downcast_ref<T: 'static>(&self) -> &T {
        self.0.downcast_ref().unwrap()
    }
}
