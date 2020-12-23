//! Private module for common code shared by multiple dialects.

use std::fmt::{self, Display, Formatter};

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
