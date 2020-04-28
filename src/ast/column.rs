use super::Aliasable;
use crate::ast::{Expression, ExpressionKind, Table};
use std::borrow::Cow;

/// A column definition.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Column<'a> {
    pub name: Cow<'a, str>,
    pub(crate) table: Option<Table<'a>>,
    pub(crate) alias: Option<Cow<'a, str>>,
}

#[macro_export]
/// Marks a given string or a tuple as a column. Useful when using a column in
/// calculations, e.g.
///
/// ``` rust
/// # use quaint::{col, val, ast::*, visitor::{Visitor, Sqlite}};
/// let join = "dogs".on(("dogs", "slave_id").equals(Column::from(("cats", "master_id"))));
///
/// let query = Select::from_table("cats")
///     .value(Table::from("cats").asterisk())
///     .value(col!("dogs", "age") - val!(4))
///     .inner_join(join);
///
/// let (sql, params) = Sqlite::build(query);
///
/// assert_eq!(
///     "SELECT `cats`.*, (`dogs`.`age` - ?) FROM `cats` INNER JOIN `dogs` ON `dogs`.`slave_id` = `cats`.`master_id`",
///     sql
/// );
/// ```
macro_rules! col {
    ($e1:expr) => {
        Expression::from(Column::from($e1))
    };

    ($e1:expr, $e2:expr) => {
        Expression::from(Column::from(($e1, $e2)))
    };
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
