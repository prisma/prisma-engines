use sql_schema_describer::walkers::*;
use sql_schema_describer::*;
use std::fmt::{Display, Write as _};

pub(super) const SQL_INDENTATION: &'static str = "    ";

#[derive(Debug)]
pub(crate) enum Quoted<T> {
    Double(T),
    Single(T),
    Backticks(T),
    SquareBrackets(T),
}

impl<T> Quoted<T> {
    fn quote_that_too<U>(&self, u: U) -> Quoted<U> {
        match self {
            Quoted::Double(_) => Quoted::Double(u),
            Quoted::Single(_) => Quoted::Single(u),
            Quoted::Backticks(_) => Quoted::Backticks(u),
            Quoted::SquareBrackets(_) => Quoted::SquareBrackets(u),
        }
    }

    pub(crate) fn mysql_string(contents: T) -> Quoted<T> {
        Quoted::Single(contents)
    }

    pub(crate) fn mysql_ident(name: T) -> Quoted<T> {
        Quoted::Backticks(name)
    }

    pub(crate) fn postgres_string(contents: T) -> Quoted<T> {
        Quoted::Single(contents)
    }

    pub(crate) fn postgres_ident(name: T) -> Quoted<T> {
        Quoted::Double(name)
    }

    pub(crate) fn sqlite_ident(name: T) -> Quoted<T> {
        Quoted::Double(name)
    }

    pub(crate) fn mssql_ident(name: T) -> Quoted<T> {
        Quoted::SquareBrackets(name)
    }
}

impl<T> Display for Quoted<T>
where
    T: Display,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Quoted::Double(inner) => write!(f, "\"{}\"", inner),
            Quoted::Single(inner) => write!(f, "'{}'", inner),
            Quoted::Backticks(inner) => write!(f, "`{}`", inner),
            Quoted::SquareBrackets(inner) => write!(f, "[{}]", inner),
        }
    }
}

#[derive(Debug)]
pub(crate) struct QuotedWithSchema<'a, T> {
    pub(crate) schema_name: &'a str,
    pub(crate) name: Quoted<T>,
}

impl<'a, T> Display for QuotedWithSchema<'a, T>
where
    T: Display,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let quoted_schema = self.name.quote_that_too(self.schema_name);

        write!(f, "{}.{}", quoted_schema, self.name)
    }
}

pub(crate) fn render_nullability(column: &ColumnWalker<'_>) -> &'static str {
    if column.arity().is_required() {
        " NOT NULL"
    } else {
        ""
    }
}

pub(crate) fn render_on_delete(on_delete: &ForeignKeyAction) -> &'static str {
    match on_delete {
        ForeignKeyAction::NoAction => "",
        ForeignKeyAction::SetNull => "ON DELETE SET NULL",
        ForeignKeyAction::Cascade => "ON DELETE CASCADE",
        ForeignKeyAction::SetDefault => "ON DELETE SET DEFAULT",
        ForeignKeyAction::Restrict => "ON DELETE RESTRICT",
    }
}

pub(crate) fn render_on_update(on_update: &ForeignKeyAction) -> &'static str {
    match on_update {
        ForeignKeyAction::NoAction => "",
        ForeignKeyAction::SetNull => "ON UPDATE SET NULL",
        ForeignKeyAction::Cascade => "ON UPDATE CASCADE",
        ForeignKeyAction::SetDefault => "ON UPDATE SET DEFAULT",
        ForeignKeyAction::Restrict => "ON UPDATE RESTRICT",
    }
}

pub(crate) trait IteratorJoin {
    fn join(self, sep: &str) -> String;
}

impl<T, I> IteratorJoin for T
where
    T: Iterator<Item = I>,
    I: Display,
{
    fn join(mut self, sep: &str) -> String {
        let (lower_bound, _) = self.size_hint();
        let mut out = String::with_capacity(sep.len() * lower_bound);

        if let Some(first_item) = self.next() {
            write!(out, "{}", first_item).unwrap();
        }

        for item in self {
            out.push_str(sep);
            write!(out, "{}", item).unwrap();
        }

        out
    }
}
