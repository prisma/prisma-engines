mod table_default;
mod view_default;

use prisma_value::PrismaValue;
pub use table_default::TableDefaultValueWalker;
pub use view_default::ViewDefaultValueWalker;

use either::Either;

use crate::{DefaultKind, DefaultValue, TableDefaultValueId, ViewDefaultValueId, Walker};

/// Traverse default value.
pub type DefaultValueWalker<'a> = Walker<'a, Either<TableDefaultValueId, ViewDefaultValueId>>;

impl<'a> DefaultValueWalker<'a> {
    /// Refines the walker to either as a table or as a view column default.
    pub fn refine(self) -> Either<TableDefaultValueWalker<'a>, ViewDefaultValueWalker<'a>> {
        match self.id {
            Either::Left(table_default) => Either::Left(self.walk(table_default)),
            Either::Right(view_default) => Either::Right(self.walk(view_default)),
        }
    }

    /// Return a value if a constant.
    pub fn as_value(self) -> Option<&'a PrismaValue> {
        match self.kind() {
            DefaultKind::Value(v) => Some(v),
            _ => None,
        }
    }

    /// If the value is a squence, return it
    pub fn as_sequence(self) -> Option<&'a str> {
        match self.kind() {
            DefaultKind::Sequence(name) => Some(name),
            _ => None,
        }
    }

    /// True if a constant value
    pub fn is_value(&self) -> bool {
        matches!(self.kind(), DefaultKind::Value(_))
    }

    /// True if `now()`
    pub fn is_now(&self) -> bool {
        matches!(self.kind(), DefaultKind::Now)
    }

    /// True if referencing a sequence
    pub fn is_sequence(&self) -> bool {
        matches!(self.kind(), DefaultKind::Sequence(_))
    }

    /// True if value generation is handled in the database
    pub fn is_db_generated(&self) -> bool {
        matches!(self.kind(), DefaultKind::DbGenerated(_))
    }

    /// The value kind enumerator
    pub fn kind(self) -> &'a DefaultKind {
        &self.value().kind
    }

    /// The name of the default value constraint.
    pub fn constraint_name(self) -> Option<&'a str> {
        self.value().constraint_name.as_deref()
    }

    /// The default value data
    pub fn value(self) -> &'a DefaultValue {
        match self.id {
            Either::Left(id) => &self.schema.table_default_values[id.0 as usize].1,
            Either::Right(id) => &self.schema.view_default_values[id.0 as usize].1,
        }
    }
}
