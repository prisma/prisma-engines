use either::Either;
use prisma_value::PrismaValue;

use crate::{DefaultKind, DefaultValue, TableColumnId, TableColumnWalker, TableDefaultValueId, Walker};

use super::DefaultValueWalker;

/// Traverse a table column's default value.
pub type TableDefaultValueWalker<'a> = Walker<'a, TableDefaultValueId>;

impl<'a> TableDefaultValueWalker<'a> {
    /// Coarsen the walker into a generic column default version.
    pub fn coarsen(self) -> DefaultValueWalker<'a> {
        self.walk(Either::Left(self.id))
    }

    /// The column where the default value is located.
    pub fn column(self) -> TableColumnWalker<'a> {
        self.walk(self.get().0)
    }

    /// Return a value if a constant.
    pub fn as_value(self) -> Option<&'a PrismaValue> {
        self.coarsen().as_value()
    }

    /// If the value is a squence, return it
    pub fn as_sequence(self) -> Option<&'a str> {
        self.coarsen().as_sequence()
    }

    /// True if a constant value
    pub fn is_value(&self) -> bool {
        self.coarsen().is_value()
    }

    /// True if `now()`
    pub fn is_now(&self) -> bool {
        self.coarsen().is_now()
    }

    /// True if referencing a sequence
    pub fn is_sequence(&self) -> bool {
        self.coarsen().is_sequence()
    }

    /// True if value generation is handled in the database
    pub fn is_db_generated(&self) -> bool {
        self.coarsen().is_db_generated()
    }

    /// The value kind enumerator
    pub fn kind(self) -> &'a DefaultKind {
        self.coarsen().kind()
    }

    /// The name of the default value constraint.
    pub fn constraint_name(self) -> Option<&'a str> {
        self.coarsen().constraint_name()
    }

    /// The actual value struct
    pub fn inner(self) -> &'a DefaultValue {
        self.coarsen().value()
    }

    fn get(self) -> &'a (TableColumnId, DefaultValue) {
        &self.schema.table_default_values[self.id.0 as usize]
    }
}
