// Setters are a bit untypical for rust,
// but we want to have "composeable" struct creation.

pub trait WithName {
    fn name(&self) -> &String;

    fn set_name(&mut self, name: &str); //Todo do not take a ref
}

pub trait WithDatabaseName {
    /// Should not be used on fields as those can have multiple db names.
    fn single_database_name(&self) -> Option<&str> {
        let db_names = self.database_names();
        if db_names.len() > 1 {
            panic!("This function must not be called on compound database names.")
        }
        db_names.into_iter().next()
    }

    fn database_names(&self) -> Vec<&str>;

    fn set_database_names(&mut self, database_name: Vec<String>);
}

pub trait Parsable: Sized {
    fn parse(s: &str) -> Option<Self>;

    fn descriptor() -> &'static str;
}
