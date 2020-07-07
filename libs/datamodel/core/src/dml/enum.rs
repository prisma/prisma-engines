use super::*;

/// Represents an enum in the datamodel.
#[derive(Debug, PartialEq, Clone)]
pub struct Enum {
    /// Name of the enum.
    pub name: String,
    /// Values of the enum.
    //    todo this needs to be able to hold database names for enum values
    pub values: Vec<EnumValue>,
    /// Comments for this enum.
    pub documentation: Option<String>,
    /// Database internal name of this enum.
    pub database_name: Option<String>,
}

impl Enum {
    /// Creates a new enum with the given name and values.
    pub fn new(name: &str, values: Vec<EnumValue>) -> Enum {
        Enum {
            name: String::from(name),
            values,
            documentation: None,
            database_name: None,
        }
    }

    pub fn add_value(&mut self, value: EnumValue) {
        self.values.push(value)
    }

    /// Gets an iterator over all fields.
    pub fn values(&self) -> std::slice::Iter<EnumValue> {
        self.values.iter()
    }

    /// Gets a mutable iterator over all fields.
    pub fn values_mut(&mut self) -> std::slice::IterMut<EnumValue> {
        self.values.iter_mut()
    }

    /// Gets an iterator over all fields.
    pub fn database_values(&self) -> Vec<String> {
        self.values
            .iter()
            .map(|v| v.database_name.as_ref().unwrap_or(&v.name).to_owned())
            .collect()
    }

    pub fn find_value(&self, value: &str) -> Option<&EnumValue> {
        self.values().find(|ev| ev.name == value)
    }

    /// Finds an enum  value by database name.
    pub fn find_value_db_name(&self, db_name: &str) -> Option<&EnumValue> {
        self.values().find(|v| v.database_name == Some(db_name.to_owned()))
    }

    pub fn find_value_mut(&mut self, value: &str) -> Option<&mut EnumValue> {
        self.values_mut().find(|ev| ev.name == value)
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
    fn database_name(&self) -> Option<&str> {
        self.database_name.as_deref()
    }

    fn set_database_name(&mut self, database_name: Option<String>) {
        self.database_name = database_name;
    }
}

/// Represents a value of an enum
#[derive(Debug, PartialEq, Clone)]
pub struct EnumValue {
    /// Value as exposed by the api
    pub name: String,
    /// Actual value as defined in the database
    pub database_name: Option<String>,
    pub documentation: Option<String>,
    /// Has to be commented out.
    pub commented_out: bool,
}

impl EnumValue {
    /// Creates a new enum value with the given name and database name.
    pub fn new(name: &str, database_name: Option<&str>) -> EnumValue {
        EnumValue {
            name: String::from(name),
            database_name: database_name.map(String::from),
            documentation: None,
            commented_out: false,
        }
    }

    /// The effective database name, i.e. the name in the @map annotation, and failing that the
    /// identifier name.
    pub fn final_database_name(&self) -> &str {
        self.database_name.as_deref().unwrap_or(self.name.as_str())
    }
}

impl WithName for EnumValue {
    fn name(&self) -> &String {
        &self.name
    }
    fn set_name(&mut self, name: &str) {
        self.name = String::from(name)
    }
}

impl WithDatabaseName for EnumValue {
    fn database_name(&self) -> Option<&str> {
        self.database_name.as_deref()
    }

    fn set_database_name(&mut self, database_name: Option<String>) {
        self.database_name = database_name;
    }
}
