use super::{Column, Comparable, ConditionTree, DefaultValue, ExpressionKind, IndexDefinition, Join, JoinData};
use crate::{
    ast::{Expression, Row, Select, Values},
    error::{Error, ErrorKind},
};
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
    JoinedTable(Box<(Cow<'a, str>, Vec<Join<'a>>)>),
    Query(Box<Select<'a>>),
    Values(Values<'a>),
}

/// A table definition
#[derive(Clone, Debug)]
pub struct Table<'a> {
    pub typ: TableType<'a>,
    pub alias: Option<Cow<'a, str>>,
    pub database: Option<Cow<'a, str>>,
    pub(crate) index_definitions: Vec<IndexDefinition<'a>>,
}

impl<'a> PartialEq for Table<'a> {
    fn eq(&self, other: &Table) -> bool {
        self.typ == other.typ && self.database == other.database
    }
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

    /// Add unique index definition.
    pub fn add_unique_index(mut self, i: impl Into<IndexDefinition<'a>>) -> Self {
        let definition = i.into();

        // FIXME: the whole circular dependency (Table -> IndexDefinition -> Column), and cloning
        // of tables inside each column.
        //
        // We can't clone `self` here, as that would lead to cycles. The following happened:
        // `add_unique_index()` clones the table, including all previous index definitions, and
        // adds the cloned table to the new index definition. On models with multiple unique
        // indexes/criterias (including PKs), we repeatedly called `add_unique_index()`. Each time,
        // we clone one more index, that itself contains copies of all previous indexes. Each
        // column in each of these previous indexes contains a partial copy of the `Table` with
        // the indexes on the table at the point of the `add_unique_index()` call that created that
        // copy.
        //
        // If we make the simplifying assumption that all the indexes have a single column, one
        // call to `add_unique_index()` would cause `(index_definitions.len() + 1)!` clones and
        // allocations of `Table`s with `IndexDefinition` arrays. That quickly leads to exhausting
        // available memory. With multiple columns per index, that adds a factor to each step in
        // the factorial.
        //
        // For symptoms of the previous naive clone, see
        // https://github.com/prisma/prisma/issues/20799 and the corresponding regression test in
        // connector-test-kit.
        let table = Table {
            typ: self.typ.clone(),
            alias: self.alias.clone(),
            database: self.database.clone(),
            index_definitions: Vec::new(),
        };

        self.index_definitions.push(definition.set_table(table));
        self
    }

    /// Conditions for Microsoft T-SQL MERGE using the table metadata.
    ///
    /// - Find the unique indices from the table that matches the inserted columns
    /// - Create a join from the virtual table with the uniques
    /// - Combine joins with `OR`
    /// - If the the index is a compound with other columns, combine them with `AND`
    /// - If the column is not provided and index exists, try inserting a default value.
    /// - Otherwise the function will return an error.
    pub(crate) fn join_conditions(&self, inserted_columns: &[Column<'a>]) -> crate::Result<ConditionTree<'a>> {
        let mut result = ConditionTree::NegativeCondition;

        let join_cond = |column: &Column<'a>| {
            let cond = if !inserted_columns.contains(column) {
                match column.default.clone() {
                    Some(DefaultValue::Provided(val)) => Some(column.clone().equals(val).into()),
                    Some(DefaultValue::Generated) => None,
                    None => {
                        let kind =
                            ErrorKind::conversion("A unique column missing from insert and table has no default.");

                        return Err(Error::builder(kind).build());
                    }
                }
            } else {
                let dual_col = column.clone().table("dual");
                Some(dual_col.equals(column.clone()).into())
            };

            Ok::<Option<ConditionTree>, Error>(cond)
        };

        for index in self.index_definitions.iter() {
            match index {
                IndexDefinition::Single(column) => {
                    if let Some(right_cond) = join_cond(column)? {
                        match result {
                            ConditionTree::NegativeCondition => result = right_cond,
                            left_cond => result = left_cond.or(right_cond),
                        }
                    }
                }
                IndexDefinition::Compound(cols) => {
                    let mut sub_result = ConditionTree::NoCondition;

                    for right in cols.iter() {
                        let right_cond = join_cond(right)?.unwrap_or(ConditionTree::NegativeCondition);

                        match sub_result {
                            ConditionTree::NoCondition => sub_result = right_cond,
                            left_cond => sub_result = left_cond.and(right_cond),
                        }
                    }

                    match result {
                        ConditionTree::NegativeCondition => result = sub_result,
                        left_cond => result = left_cond.or(sub_result),
                    }
                }
            }
        }

        Ok(result)
    }

    /// Adds a `LEFT JOIN` clause to the query, specifically for that table.
    /// Useful to positionally add a JOIN clause in case you are selecting from multiple tables.
    ///
    /// ```rust
    /// # use quaint::{ast::*, visitor::{Visitor, Sqlite}};
    /// # fn main() -> Result<(), quaint::error::Error> {
    /// let join = "posts".alias("p").on(("p", "visible").equals(true));
    /// let joined_table = Table::from("users").left_join(join);
    /// let query = Select::from_table(joined_table).and_from("comments");
    /// let (sql, params) = Sqlite::build(query)?;
    ///
    /// assert_eq!(
    ///     "SELECT `users`.*, `comments`.* FROM \
    ///     `users` LEFT JOIN `posts` AS `p` ON `p`.`visible` = ?, \
    ///     `comments`",
    ///     sql
    /// );
    ///
    /// assert_eq!(
    ///     vec![
    ///         Value::from(true),
    ///     ],
    ///     params
    /// );
    /// # Ok(())
    /// # }
    /// ```
    pub fn left_join<J>(mut self, join: J) -> Self
    where
        J: Into<JoinData<'a>>,
    {
        match self.typ {
            TableType::Table(table_name) => {
                self.typ = TableType::JoinedTable(Box::new((table_name, vec![Join::Left(join.into())])))
            }
            TableType::JoinedTable(ref mut jt) => jt.1.push(Join::Left(join.into())),
            TableType::Query(_) => {
                panic!("You cannot left_join on a table of type Query")
            }
            TableType::Values(_) => {
                panic!("You cannot left_join on a table of type Values")
            }
        }

        self
    }

    /// Adds an `INNER JOIN` clause to the query, specifically for that table.
    /// Useful to positionally add a JOIN clause in case you are selecting from multiple tables.
    ///
    /// ```rust
    /// # use quaint::{ast::*, visitor::{Visitor, Sqlite}};
    /// # fn main() -> Result<(), quaint::error::Error> {
    /// let join = "posts".alias("p").on(("p", "visible").equals(true));
    /// let joined_table = Table::from("users").inner_join(join);
    /// let query = Select::from_table(joined_table).and_from("comments");
    /// let (sql, params) = Sqlite::build(query)?;
    ///
    /// assert_eq!(
    ///     "SELECT `users`.*, `comments`.* FROM \
    ///     `users` INNER JOIN `posts` AS `p` ON `p`.`visible` = ?, \
    ///     `comments`",
    ///     sql
    /// );
    ///
    /// assert_eq!(
    ///     vec![
    ///         Value::from(true),
    ///     ],
    ///     params
    /// );
    /// # Ok(())
    /// # }
    /// ```
    pub fn inner_join<J>(mut self, join: J) -> Self
    where
        J: Into<JoinData<'a>>,
    {
        match self.typ {
            TableType::Table(table_name) => {
                self.typ = TableType::JoinedTable(Box::new((table_name, vec![Join::Inner(join.into())])))
            }
            TableType::JoinedTable(ref mut jt) => jt.1.push(Join::Inner(join.into())),
            TableType::Query(_) => {
                panic!("You cannot inner_join on a table of type Query")
            }
            TableType::Values(_) => {
                panic!("You cannot inner_join on a table of type Values")
            }
        }

        self
    }

    /// Adds a `RIGHT JOIN` clause to the query, specifically for that table.
    /// Useful to positionally add a JOIN clause in case you are selecting from multiple tables.
    ///
    /// ```rust
    /// # use quaint::{ast::*, visitor::{Visitor, Sqlite}};
    /// # fn main() -> Result<(), quaint::error::Error> {
    /// let join = "posts".alias("p").on(("p", "visible").equals(true));
    /// let joined_table = Table::from("users").right_join(join);
    /// let query = Select::from_table(joined_table).and_from("comments");
    /// let (sql, params) = Sqlite::build(query)?;
    ///
    /// assert_eq!(
    ///     "SELECT `users`.*, `comments`.* FROM \
    ///     `users` RIGHT JOIN `posts` AS `p` ON `p`.`visible` = ?, \
    ///     `comments`",
    ///     sql
    /// );
    ///
    /// assert_eq!(
    ///     vec![
    ///         Value::from(true),
    ///     ],
    ///     params
    /// );
    /// # Ok(())
    /// # }
    /// ```
    pub fn right_join<J>(mut self, join: J) -> Self
    where
        J: Into<JoinData<'a>>,
    {
        match self.typ {
            TableType::Table(table_name) => {
                self.typ = TableType::JoinedTable(Box::new((table_name, vec![Join::Right(join.into())])))
            }
            TableType::JoinedTable(ref mut jt) => jt.1.push(Join::Right(join.into())),
            TableType::Query(_) => {
                panic!("You cannot right_join on a table of type Query")
            }
            TableType::Values(_) => {
                panic!("You cannot right_join on a table of type Values")
            }
        }

        self
    }

    /// Adds a `FULL JOIN` clause to the query, specifically for that table.
    /// Useful to positionally add a JOIN clause in case you are selecting from multiple tables.
    ///
    /// ```rust
    /// # use quaint::{ast::*, visitor::{Visitor, Sqlite}};
    /// # fn main() -> Result<(), quaint::error::Error> {
    /// let join = "posts".alias("p").on(("p", "visible").equals(true));
    /// let joined_table = Table::from("users").full_join(join);
    /// let query = Select::from_table(joined_table).and_from("comments");
    /// let (sql, params) = Sqlite::build(query)?;
    ///
    /// assert_eq!(
    ///     "SELECT `users`.*, `comments`.* FROM \
    ///     `users` FULL JOIN `posts` AS `p` ON `p`.`visible` = ?, \
    ///     `comments`",
    ///     sql
    /// );
    ///
    /// assert_eq!(
    ///     vec![
    ///         Value::from(true),
    ///     ],
    ///     params
    /// );
    /// # Ok(())
    /// # }
    /// ```
    pub fn full_join<J>(mut self, join: J) -> Self
    where
        J: Into<JoinData<'a>>,
    {
        match self.typ {
            TableType::Table(table_name) => {
                self.typ = TableType::JoinedTable(Box::new((table_name, vec![Join::Full(join.into())])))
            }
            TableType::JoinedTable(ref mut jt) => jt.1.push(Join::Full(join.into())),
            TableType::Query(_) => {
                panic!("You cannot full_join on a table of type Query")
            }
            TableType::Values(_) => {
                panic!("You cannot full_join on a table of type Values")
            }
        }

        self
    }

    pub fn join<J>(self, join: J) -> Self
    where
        J: Into<Join<'a>>,
    {
        let join: Join = join.into();

        match join {
            Join::Inner(x) => self.inner_join(x),
            Join::Left(x) => self.left_join(x),
            Join::Right(x) => self.right_join(x),
            Join::Full(x) => self.full_join(x),
        }
    }
}

impl<'a> From<&'a str> for Table<'a> {
    fn from(s: &'a str) -> Table<'a> {
        Table {
            typ: TableType::Table(s.into()),
            alias: None,
            database: None,
            index_definitions: Vec::new(),
        }
    }
}

impl<'a> From<&'a String> for Table<'a> {
    fn from(s: &'a String) -> Table<'a> {
        Table {
            typ: TableType::Table(s.into()),
            alias: None,
            database: None,
            index_definitions: Vec::new(),
        }
    }
}

impl<'a> From<(&'a str, &'a str)> for Table<'a> {
    fn from(s: (&'a str, &'a str)) -> Table<'a> {
        let table: Table<'a> = s.1.into();
        table.database(s.0)
    }
}

impl<'a> From<(&'a str, &'a String)> for Table<'a> {
    fn from(s: (&'a str, &'a String)) -> Table<'a> {
        let table: Table<'a> = s.1.into();
        table.database(s.0)
    }
}

impl<'a> From<(&'a String, &'a str)> for Table<'a> {
    fn from(s: (&'a String, &'a str)) -> Table<'a> {
        let table: Table<'a> = s.1.into();
        table.database(s.0)
    }
}

impl<'a> From<(&'a String, &'a String)> for Table<'a> {
    fn from(s: (&'a String, &'a String)) -> Table<'a> {
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
            index_definitions: Vec::new(),
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
            index_definitions: Vec::new(),
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
            typ: TableType::Query(Box::new(select)),
            alias: None,
            database: None,
            index_definitions: Vec::new(),
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

aliasable!(String, (String, String));
aliasable!(&'a str, (&'a str, &'a str));
