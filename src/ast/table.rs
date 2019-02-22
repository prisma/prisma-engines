#[derive(Clone, Debug, Default, PartialEq)]
pub struct Table {
    pub name: String,
    pub database: Option<String>,
}

impl Table {
    pub fn database<T>(mut self, database: T) -> Self
    where
        T: Into<String>,
    {
        self.database = Some(database.into());
        self
    }
}

impl<'a> Into<Table> for &'a str {
    fn into(self) -> Table {
        Table {
            name: self.to_string(),
            database: None,
        }
    }
}

impl<'a, 'b> Into<Table> for (&'a str, &'b str) {
    fn into(self) -> Table {
        let table: Table = self.1.into();
        table.database(self.0)
    }
}
