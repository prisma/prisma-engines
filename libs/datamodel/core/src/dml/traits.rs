use std::result::Result;

pub trait WithName {
    fn name(&self) -> &String;

    fn set_name(&mut self, name: &str); //Todo do not take a ref
}

pub trait WithDatabaseName: WithName {
    /// Should not be used on fields as those can have multiple db names.
    fn single_database_name(&self) -> Option<&str> {
        let db_names = self.database_names();
        if db_names.len() > 1 {
            panic!("This function must not be called on compound database names.")
        }
        db_names.into_iter().next()
    }

    fn final_single_database_name(&self) -> &str {
        self.single_database_name().unwrap_or(self.name())
    }

    fn database_names(&self) -> Vec<&str>;

    fn set_database_names(&mut self, database_name: Vec<String>) -> Result<(), String>;

    fn set_database_name(&mut self, database_name: Option<String>);
}

pub trait Parsable: Sized {
    fn parse(s: &str) -> Option<Self>;

    fn descriptor() -> &'static str;
}
