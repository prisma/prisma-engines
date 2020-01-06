use super::*;

/// Represents an enum in the datamodel.
#[derive(Debug, PartialEq, Clone)]
pub struct Enum {
    /// Name of the enum.
    pub name: String,
    /// Values of the enum.
    pub values: Vec<String>,
    /// Comments for this enum.
    pub documentation: Option<String>,
    /// Database internal name of this enum.
    pub database_name: Option<DatabaseName>,
}

impl Enum {
    /// Creates a new enum with the given name and values.
    pub fn new(name: &str, values: Vec<String>) -> Enum {
        Enum {
            name: String::from(name),
            values,
            documentation: None,
            database_name: None,
        }
    }
}

impl WithName for Enum {
    fn name(&self) -> &String {
        &self.name
    }
    fn set_name(&mut self, name: &str) {
        self.name = String::from(name)
    }
}

impl WithDatabaseName for Enum {
    fn single_database_name(&self) -> Option<&str> {
        match &self.database_name {
            None => None,
            Some(DatabaseName::Single(name)) => Some(name.as_ref()),
            Some(DatabaseName::Compound(_)) => panic!("Enums should not have compound databasenames."),
        }
    }

    fn database_names(&self) -> &Option<DatabaseName> {
        &self.database_name
    }

    fn set_database_names(&mut self, database_name: Option<DatabaseName>) {
        self.database_name = database_name
    }
}
