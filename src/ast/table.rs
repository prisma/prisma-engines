use super::ExpressionKind;
use crate::ast::{Expression, Row, Select, Values};
use std::borrow::Cow;

/// An object that can be aliased.
pub trait Aliasable<'a> {
    type Target;

    /// Alias table for usage elsewhere in the query.
    fn alias<T>(self, alias: T) -> Self::Target
    where
        T: Into<Cow<'a, str>>;
}

#[derive(Clone, Debug, PartialEq)]
/// Either an identifier or a nested query.
pub enum TableType<'a> {
    Table(Cow<'a, str>),
    Query(Select<'a>),
    Values(Values<'a>),
}

/// A table definition
#[derive(Clone, Debug, PartialEq)]
pub struct Table<'a> {
    pub typ: TableType<'a>,
    pub alias: Option<Cow<'a, str>>,
    pub database: Option<Cow<'a, str>>,
}

impl<'a> Table<'a> {
    /// Define in which database the table is located
    pub fn database<T>(mut self, database: T) -> Self
    where
        T: Into<Cow<'a, str>>,
    {
        self.database = Some(database.into());
        self
    }

    /// A qualified asterisk to this table
    pub fn asterisk(self) -> Expression<'a> {
        Expression {
            kind: ExpressionKind::Asterisk(Some(Box::new(self))),
            alias: None,
        }
    }
}

impl<'a> From<&'a str> for Table<'a> {
    fn from(s: &'a str) -> Table<'a> {
        Table {
            typ: TableType::Table(s.into()),
            alias: None,
            database: None,
        }
    }
}

impl<'a> From<(&'a str, &'a str)> for Table<'a> {
    fn from(s: (&'a str, &'a str)) -> Table<'a> {
        let table: Table<'a> = s.1.into();
        table.database(s.0)
    }
}

impl<'a> From<String> for Table<'a> {
    fn from(s: String) -> Self {
        Table {
            typ: TableType::Table(s.into()),
            alias: None,
            database: None,
        }
    }
}

impl<'a> From<Vec<Row<'a>>> for Table<'a> {
    fn from(values: Vec<Row<'a>>) -> Self {
        Table::from(Values::from(values.into_iter()))
    }
}

impl<'a> From<Values<'a>> for Table<'a> {
    fn from(values: Values<'a>) -> Self {
        Self {
            typ: TableType::Values(values),
            alias: None,
            database: None,
        }
    }
}

impl<'a> From<(String, String)> for Table<'a> {
    fn from(s: (String, String)) -> Table<'a> {
        let table: Table<'a> = s.1.into();
        table.database(s.0)
    }
}

impl<'a> From<Select<'a>> for Table<'a> {
    fn from(select: Select<'a>) -> Self {
        Table {
            typ: TableType::Query(select),
            alias: None,
            database: None,
        }
    }
}

impl<'a> Aliasable<'a> for Table<'a> {
    type Target = Table<'a>;

    fn alias<T>(mut self, alias: T) -> Self::Target
    where
        T: Into<Cow<'a, str>>,
    {
        self.alias = Some(alias.into());
        self
    }
}

macro_rules! aliasable {
    ($($kind:ty),*) => (
        $(
            impl<'a> Aliasable<'a> for $kind {
                type Target = Table<'a>;

                fn alias<T>(self, alias: T) -> Self::Target
                where
                    T: Into<Cow<'a, str>>,
                {
                    let table: Table = self.into();
                    table.alias(alias)
                }
            }
        )*
    );
}

aliasable!(String, (String, String));
aliasable!(&'a str, (&'a str, &'a str));
