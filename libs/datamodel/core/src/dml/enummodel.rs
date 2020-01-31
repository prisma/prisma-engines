use super::*;

/// Represents an enum in the datamodel.
#[derive(Debug, PartialEq, Clone)]
pub struct Enum {
    /// Name of the enum.
    pub name: String,
    /// Values of the enum.
    //todo this needs to be able to hold database names for enum values -> tuple? or struct?
    //struct could implement WithDatabaseName and using existing traits
    pub values: Vec<String>,
    /// Comments for this enum.
    pub documentation: Option<String>,
    /// Database internal name of this enum.
    pub database_name: Option<String>,
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
    fn database_names(&self) -> Vec<&str> {
        match &self.database_name {
            None => vec![],
            Some(db_name) => vec![db_name],
        }
    }

    fn set_database_names(&mut self, database_names: Vec<String>) -> Result<(), String> {
        if database_names.len() > 1 {
            Err("An Enum must not specify multiple mapped names.".to_string())
        } else {
            let first = database_names.into_iter().next();
            self.database_name = first;
            Ok(())
        }
    }
}
