/// A boxed migration, opaque to the schema engine core. The connectors are
/// sole responsible for producing and understanding migrations — the core just
/// orchestrates.
pub struct Migration(Box<dyn std::any::Any + Send + Sync>);

impl Migration {
    /// Type-erase a migration.
    pub fn new<T: 'static + Send + Sync>(migration: T) -> Self {
        Migration(Box::new(migration))
    }

    /// Should never be used in the core, only in connectors that know what they put there.
    pub fn downcast_ref<T: 'static>(&self) -> &T {
        self.0.downcast_ref().unwrap()
    }
}
