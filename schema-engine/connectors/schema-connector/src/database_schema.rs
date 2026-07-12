/// A trait for database schema values ensuring that it's possible to clone a DatabaseSchema.
pub trait DatabaseSchemaValue: std::any::Any + Send + Sync + 'static {
    fn clone_box(&self) -> Box<dyn DatabaseSchemaValue>;
}

impl<T: 'static + Send + Sync + Clone> DatabaseSchemaValue for T {
    fn clone_box(&self) -> Box<dyn DatabaseSchemaValue> {
        Box::new(self.clone())
    }
}

/// A database schema. Part of the MigrationConnector API.
pub struct DatabaseSchema(Box<dyn DatabaseSchemaValue>);

impl Clone for DatabaseSchema {
    fn clone(&self) -> Self {
        DatabaseSchema(self.0.clone_box())
    }
}

impl DatabaseSchema {
    /// Type-erase a migration.
    pub fn new<T: 'static + Send + Sync + Clone>(migration: T) -> Self {
        DatabaseSchema(Box::new(migration))
    }

    /// Should never be used in the core, only in connectors that know what they put there.
    pub fn downcast<T: 'static>(self) -> Box<T> {
        let any: Box<dyn std::any::Any> = self.0;
        any.downcast().unwrap()
    }

    /// Should never be used in the core, only in connectors that know what they put there.
    pub fn downcast_ref<T: 'static>(&self) -> &T {
        let any: &dyn std::any::Any = &*self.0;
        any.downcast_ref().unwrap()
    }
}
