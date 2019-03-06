use crate::ast::{ConditionTree, JoinData, Joinable, Select};

/// An object that can be aliased.
pub trait Aliasable {
    /// Alias table for usage elsewhere in the query.
    fn alias<T>(self, alias: T) -> Table
    where
        T: Into<String>;
}

#[derive(Clone, Debug, PartialEq)]
pub enum TableType {
    Table(String),
    Query(Select),
}

/// A table definition
#[derive(Clone, Debug, PartialEq)]
pub struct Table {
    pub typ: TableType,
    pub alias: Option<String>,
    pub database: Option<String>,
}

impl Table {
    /// Define in which database the table is located
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
            typ: TableType::Table(self.to_string()),
            alias: None,
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

impl Into<Table> for String {
    fn into(self) -> Table {
        Table {
            typ: TableType::Table(self),
            alias: None,
            database: None,
        }
    }
}

impl Into<Table> for (String, String) {
    fn into(self) -> Table {
        let table: Table = self.1.into();
        table.database(self.0)
    }
}

impl From<Select> for Table {
    fn from(select: Select) -> Table {
        Table {
            typ: TableType::Query(select),
            alias: None,
            database: None,
        }
    }
}

impl Joinable for Table {
    fn on<T>(self, conditions: T) -> JoinData
    where
        T: Into<ConditionTree>,
    {
        JoinData {
            table: self,
            conditions: conditions.into(),
        }
    }
}

impl Aliasable for Table {
    fn alias<T>(mut self, alias: T) -> Self
    where
        T: Into<String>,
    {
        self.alias = Some(alias.into());
        self
    }
}

macro_rules! aliasable {
    ($($kind:ty),*) => (
        $(
            impl Aliasable for $kind {
                fn alias<T>(self, alias: T) -> Table
                where
                    T: Into<String>,
                {
                    let table: Table = self.into();
                    table.alias(alias)
                }
            }
        )*
    );
}

aliasable!(String, (String, String));
aliasable!(&str, (&str, &str));
