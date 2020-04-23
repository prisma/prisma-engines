pub trait WithName {
    fn name(&self) -> &String;

    fn set_name(&mut self, name: &str); //Todo do not take a ref
}

pub trait WithDatabaseName: WithName {
    fn database_name(&self) -> Option<&str>;

    fn final_database_name(&self) -> &str {
        self.database_name().unwrap_or(self.name())
    }

    fn set_database_name(&mut self, database_name: Option<String>);
}

pub trait Parsable: Sized {
    fn parse(s: &str) -> Option<Self>;

    fn descriptor() -> &'static str;
}
