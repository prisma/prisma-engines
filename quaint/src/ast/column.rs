use super::Aliasable;
use crate::{
    ast::{Expression, ExpressionKind, Table},
    Value,
};
use std::borrow::Cow;

#[derive(Debug, Clone, Copy)]
pub enum TypeDataLength {
    Constant(u16),
    Maximum,
}

#[derive(Debug, Clone, Copy)]
pub enum TypeFamily {
    Text(Option<TypeDataLength>),
    Int,
    Float,
    Double,
    Boolean,
    Uuid,
    DateTime,
    Decimal(Option<(u8, u8)>),
    Bytes(Option<TypeDataLength>),
}

/// A column definition.
#[derive(Clone, Debug, Default)]
pub struct Column<'a> {
    pub name: Cow<'a, str>,
    pub(crate) table: Option<Table<'a>>,
    pub(crate) alias: Option<Cow<'a, str>>,
    pub(crate) default: Option<DefaultValue<'a>>,
    pub(crate) type_family: Option<TypeFamily>,
}

/// Defines a default value for a `Column`.
#[derive(Clone, Debug, PartialEq)]
pub enum DefaultValue<'a> {
    /// A static value.
    Provided(Value<'a>),
    /// Generated in the database.
    Generated,
}

impl<'a> Default for DefaultValue<'a> {
    fn default() -> Self {
        Self::Generated
    }
}

impl<'a, V> From<V> for DefaultValue<'a>
where
    V: Into<Value<'a>>,
{
    fn from(v: V) -> Self {
        Self::Provided(v.into())
    }
}

impl<'a> PartialEq for Column<'a> {
    fn eq(&self, other: &Column) -> bool {
        self.name == other.name && self.table == other.table
    }
}

impl<'a> Column<'a> {
    /// Create a bare version of the column, stripping out all other information
    /// other than the name.
    pub(crate) fn into_bare(self) -> Self {
        Self {
            name: self.name,
            ..Default::default()
        }
    }

    /// Sets the default value for the column.
    pub fn default<V>(mut self, value: V) -> Self
    where
        V: Into<DefaultValue<'a>>,
    {
        self.default = Some(value.into());
        self
    }

    /// Sets a type family, used mainly for SQL Server `OUTPUT` hack.
    pub fn type_family(mut self, type_family: TypeFamily) -> Self {
        self.type_family = Some(type_family);
        self
    }

    /// True when the default value is set and automatically generated in the
    /// database.
    pub fn default_autogen(&self) -> bool {
        self.default
            .as_ref()
            .map(|d| d == &DefaultValue::Generated)
            .unwrap_or(false)
    }
}

impl<'a> From<Column<'a>> for Expression<'a> {
    fn from(col: Column<'a>) -> Self {
        Expression {
            kind: ExpressionKind::Column(Box::new(col)),
            alias: None,
        }
    }
}

impl<'a> Column<'a> {
    /// Create a column definition.
    pub fn new<S>(name: S) -> Self
    where
        S: Into<Cow<'a, str>>,
    {
        Column {
            name: name.into(),
            ..Default::default()
        }
    }

    /// Include the table name in the column expression.
    pub fn table<T>(mut self, table: T) -> Self
    where
        T: Into<Table<'a>>,
    {
        self.table = Some(table.into());
        self
    }

    /// Include the table name in the column expression, if table is defined.
    pub fn opt_table<T>(mut self, table: Option<T>) -> Self
    where
        T: Into<Table<'a>>,
    {
        if let Some(table) = table {
            self.table = Some(table.into());
        }

        self
    }
}

impl<'a> Aliasable<'a> for Column<'a> {
    type Target = Column<'a>;

    fn alias<T>(mut self, alias: T) -> Self::Target
    where
        T: Into<Cow<'a, str>>,
    {
        self.alias = Some(alias.into());
        self
    }
}

impl<'a> From<&'a str> for Column<'a> {
    fn from(s: &'a str) -> Self {
        Column {
            name: s.into(),
            ..Default::default()
        }
    }
}

impl<'a, 'b> From<&'a &'b str> for Column<'b> {
    fn from(s: &'a &'b str) -> Self {
        Column::from(*s)
    }
}

impl<'a> From<String> for Column<'a> {
    fn from(s: String) -> Self {
        Column {
            name: s.into(),
            ..Default::default()
        }
    }
}

impl<'a, T, C> From<(T, C)> for Column<'a>
where
    T: Into<Table<'a>>,
    C: Into<Column<'a>>,
{
    fn from(t: (T, C)) -> Column<'a> {
        let mut column: Column<'a> = t.1.into();
        column = column.table(t.0);

        column
    }
}
