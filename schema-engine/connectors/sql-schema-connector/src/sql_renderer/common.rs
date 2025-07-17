use sql_schema_describer::{walkers::TableColumnWalker, *};
use std::fmt::{Display, Write as _};

pub(crate) const SQL_INDENTATION: &str = "    ";

/// A quoted identifier with an optional schema prefix.
#[derive(Clone, Copy)]
pub(crate) struct QuotedWithPrefix<T>(pub(crate) Option<Quoted<T>>, pub(crate) Quoted<T>);

impl QuotedWithPrefix<&str> {
    pub(crate) fn pg_new<'a>(namespace: Option<&'a str>, name: &'a str) -> QuotedWithPrefix<&'a str> {
        QuotedWithPrefix(namespace.map(Quoted::postgres_ident), Quoted::postgres_ident(name))
    }

    pub(crate) fn pg_from_table_walker(table: TableWalker<'_>) -> QuotedWithPrefix<&str> {
        QuotedWithPrefix(
            table.namespace().map(Quoted::postgres_ident),
            Quoted::postgres_ident(table.name()),
        )
    }
}

impl<T> Display for QuotedWithPrefix<T>
where
    T: Display,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(schema) = &self.0 {
            Display::fmt(&schema, f)?;
            f.write_str(".")?;
        }
        Display::fmt(&self.1, f)
    }
}

#[derive(Debug, Clone, Copy)]
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
            Quoted::Double(inner) => write!(f, "\"{inner}\""),
            Quoted::Single(inner) => write!(f, "'{inner}'"),
            Quoted::Backticks(inner) => write!(f, "`{inner}`"),
            Quoted::SquareBrackets(inner) => write!(f, "[{inner}]"),
        }
    }
}

pub fn render_nullability(column: TableColumnWalker<'_>) -> &'static str {
    if column.arity().is_required() { " NOT NULL" } else { "" }
}

pub fn render_referential_action(action: ForeignKeyAction) -> &'static str {
    match action {
        ForeignKeyAction::NoAction => "NO ACTION",
        ForeignKeyAction::Restrict => "RESTRICT",
        ForeignKeyAction::Cascade => "CASCADE",
        ForeignKeyAction::SetNull => "SET NULL",
        ForeignKeyAction::SetDefault => "SET DEFAULT",
    }
}

pub fn format_hex(bytes: &[u8], out: &mut String) {
    use std::fmt::Write as _;

    out.reserve(bytes.len() * 2);

    for byte in bytes {
        write!(out, "{byte:02x}").expect("failed to hex format a byte");
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
            write!(out, "{first_item}").unwrap();
        }

        for item in self {
            out.push_str(sep);
            write!(out, "{item}").unwrap();
        }

        out
    }
}

#[derive(Default)]
pub(crate) struct StepRenderer {
    stmts: Vec<String>,
}

impl StepRenderer {
    pub fn render_statement(&mut self, f: &mut dyn FnMut(&mut StatementRenderer)) {
        let mut stmt_renderer = Default::default();
        f(&mut stmt_renderer);
        self.stmts.push(stmt_renderer.statement);
    }
}

#[derive(Default)]
pub(crate) struct StatementRenderer {
    statement: String,
}

impl StatementRenderer {
    pub fn join<I, T>(&mut self, separator: &str, iter: I)
    where
        I: Iterator<Item = T>,
        T: std::fmt::Display,
    {
        let mut iter = iter.peekable();
        while let Some(item) = iter.next() {
            self.push_display(&item);
            if iter.peek().is_some() {
                self.push_str(separator)
            }
        }
    }

    pub fn push_str(&mut self, s: &str) {
        self.statement.push_str(s)
    }

    pub fn push_display(&mut self, d: &dyn std::fmt::Display) {
        std::fmt::Write::write_fmt(&mut self.statement, format_args!("{d}")).unwrap();
    }
}

pub fn render_step(f: &mut dyn FnMut(&mut StepRenderer)) -> Vec<String> {
    let mut renderer = Default::default();
    f(&mut renderer);
    renderer.stmts
}
