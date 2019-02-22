#[derive(Clone, Debug, Default, PartialEq)]
pub struct Table {
    pub name: String,
    pub database: Option<String>,
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
        Table {
            name: self.1.to_string(),
            database: Some(self.0.to_string()),
        }
    }
}
