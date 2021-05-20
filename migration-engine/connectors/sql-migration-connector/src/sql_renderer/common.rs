use sql_schema_describer::{walkers::ColumnWalker, *};
use std::fmt::{Display, Write as _};

pub(super) const SQL_INDENTATION: &str = "    ";

#[derive(Debug)]
pub(crate) enum Quoted<T> {
    Double(T),
    Single(T),
    Backticks(T),
    SquareBrackets(T),
}

impl<T> Quoted<T> {
    pub(crate) fn mssql_string(contents: T) -> Quoted<T> {
        Quoted::Single(contents)
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

    pub(crate) fn sqlite_string(name: T) -> Quoted<T> {
        Quoted::Single(name)
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

pub(crate) fn render_nullability(column: &ColumnWalker<'_>) -> &'static str {
    if column.arity().is_required() {
        " NOT NULL"
    } else {
        ""
    }
}

pub(crate) fn render_referential_action(action: &ForeignKeyAction) -> &'static str {
    match action {
        ForeignKeyAction::NoAction => "NO ACTION",
        ForeignKeyAction::Restrict => "RESTRICT",
        ForeignKeyAction::Cascade => "CASCADE",
        ForeignKeyAction::SetNull => "SET NULL",
        ForeignKeyAction::SetDefault => "SET DEFAULT",
    }
}

pub(crate) fn format_hex(bytes: &[u8]) -> String {
    use std::fmt::Write as _;

    let mut out = String::with_capacity(bytes.len() * 2);

    for byte in bytes {
        write!(out, "{:02x}", byte).expect("failed to hex format a byte");
    }

    out
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
