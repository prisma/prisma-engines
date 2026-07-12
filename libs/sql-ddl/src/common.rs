//! Private module for common code shared by multiple dialects.

use std::{
    borrow::Cow,
    fmt::{self, Display, Formatter},
};

/// The indentation used throughout the crate. Four spaces.
pub const SQL_INDENTATION: &str = "    ";

pub(crate) struct Indented<T>(pub T);

impl<T: Display> Display for Indented<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(SQL_INDENTATION)?;
        self.0.fmt(f)
    }
}

pub(crate) trait IteratorJoin {
    fn join(self, separator: &str, f: &mut Formatter) -> fmt::Result;
}

impl<T, I> IteratorJoin for T
where
    T: Iterator<Item = I>,
    I: Display,
{
    fn join(self, separator: &str, f: &mut Formatter) -> fmt::Result {
        let mut items = self.peekable();

        while let Some(item) = items.next() {
            item.fmt(f)?;

            if items.peek().is_some() {
                f.write_str(separator)?;
            }
        }

        Ok(())
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub enum SortOrder {
    #[default]
    Asc,
    Desc,
}

impl Display for SortOrder {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_ref())
    }
}

impl AsRef<str> for SortOrder {
    fn as_ref(&self) -> &str {
        match self {
            SortOrder::Asc => "ASC",
            SortOrder::Desc => "DESC",
        }
    }
}

#[derive(Debug, Default)]
pub struct IndexColumn<'a> {
    pub name: Cow<'a, str>,
    pub length: Option<u32>,
    pub sort_order: Option<SortOrder>,
    pub operator_class: Option<String>,
}

impl<'a> IndexColumn<'a> {
    pub fn new(name: &'a str) -> Self {
        Self {
            name: name.into(),
            ..Default::default()
        }
    }
}
