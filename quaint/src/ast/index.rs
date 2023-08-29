use super::Column;

/// A definition of a database index.
///
/// Used mainly for the transformation of a `INSERT` into a `MERGE`.
#[derive(Debug, PartialEq, Clone)]
pub enum IndexDefinition<'a> {
    Single(Box<Column<'a>>),
    Compound(Vec<Column<'a>>),
}

impl<'a> IndexDefinition<'a> {
    /// At least one of the index columns has automatically generated default
    /// value in the database.
    pub fn has_autogen(&self) -> bool {
        match self {
            Self::Single(c) => c.default_autogen(),
            Self::Compound(cols) => cols.iter().any(|c| c.default_autogen()),
        }
    }

    /// True if the index definition contains the given column.
    pub fn contains(&self, column: &Column) -> bool {
        match self {
            Self::Single(ref c) if c.as_ref() == column => true,
            Self::Compound(ref cols) if cols.iter().any(|c| c == column) => true,
            _ => false,
        }
    }
}

impl<'a, T> From<T> for IndexDefinition<'a>
where
    T: Into<Column<'a>>,
{
    fn from(s: T) -> Self {
        Self::Single(Box::new(s.into()))
    }
}

impl<'a, T> From<Vec<T>> for IndexDefinition<'a>
where
    T: Into<Column<'a>>,
{
    fn from(s: Vec<T>) -> Self {
        Self::Compound(s.into_iter().map(|c| c.into()).collect())
    }
}
