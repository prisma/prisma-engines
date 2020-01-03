// Setters are a bit untypical for rust,
// but we want to have "composeable" struct creation.

/// Trait for all datamodel objects which have a name.
pub trait WithName {
    /// Gets the name.
    fn name(&self) -> &String;
    /// Sets the name.
    fn set_name(&mut self, name: &str); //Todo do not take a ref
}

#[derive(Debug, PartialEq, Clone)]
pub enum DatabaseName {
    Single(String),
    Compound(Vec<String>),
}

/// Trait for all datamodel objects which have an internal database name.
pub trait WithDatabaseName {
    /// Gets the internal database name.
    fn database_name(&self) -> &Option<String>;
    /// Gets the proper database name enum. unused for now
    fn database_names(&self) -> &Option<DatabaseName>;
    /// Sets the internal database name.
    fn set_database_name(&mut self, database_name: Option<DatabaseName>);
}

pub trait Parsable: Sized {
    fn parse(s: &str) -> Option<Self>;

    fn descriptor() -> &'static str;
}
