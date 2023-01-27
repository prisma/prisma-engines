use either::Either;

use crate::{
    Column, ColumnArity, ColumnType, ColumnTypeFamily, EnumWalker, ViewColumnId, ViewDefaultValueId,
    ViewDefaultValueWalker, ViewId, ViewWalker, Walker,
};

use super::ColumnWalker;

/// Traverse a view column.
pub type ViewColumnWalker<'a> = Walker<'a, ViewColumnId>;

impl<'a> ViewColumnWalker<'a> {
    fn get(self) -> &'a (ViewId, Column) {
        &self.schema.view_columns[self.id.0 as usize]
    }

    /// Coarsen the walker into a generic column version.
    pub fn coarsen(self) -> ColumnWalker<'a> {
        self.walk(Either::Right(self.id))
    }

    /// The column name.
    pub fn name(self) -> &'a str {
        self.coarsen().name()
    }

    /// The nullability and arity of the column.
    pub fn arity(self) -> ColumnArity {
        self.coarsen().arity()
    }

    /// Returns whether the column has the enum default value of the given enum type.
    pub fn column_has_enum_default_value(self, enum_name: &str, value: &str) -> bool {
        self.coarsen().column_has_enum_default_value(enum_name, value)
    }

    /// Returns whether the type of the column matches the provided enum name.
    pub fn column_type_is_enum(self, enum_name: &str) -> bool {
        self.coarsen().column_type_is_enum(enum_name)
    }

    /// The type family.
    pub fn column_type_family(self) -> &'a ColumnTypeFamily {
        self.coarsen().column_type_family()
    }

    /// Extract an `Enum` column type family, or `None` if the family is something else.
    pub fn column_type_family_as_enum(self) -> Option<EnumWalker<'a>> {
        self.coarsen().column_type_family_as_enum()
    }

    /// the default value for the column.
    pub fn default(self) -> Option<ViewDefaultValueWalker<'a>> {
        self.schema
            .view_default_values
            .binary_search_by_key(&self.id, |(id, _)| *id)
            .ok()
            .map(|id| self.walk(ViewDefaultValueId(id as u32)))
    }

    /// The full column type.
    pub fn column_type(self) -> &'a ColumnType {
        self.coarsen().column_type()
    }

    /// The column native type.
    pub fn column_native_type<T: std::any::Any + 'static>(self) -> Option<&'a T> {
        self.coarsen().column_native_type()
    }

    /// Is this column an auto-incrementing integer?
    pub fn is_autoincrement(self) -> bool {
        self.coarsen().is_autoincrement()
    }

    /// Returns whether two columns are named the same and belong to the same table.
    pub fn is_same_column(self, other: ViewColumnWalker<'_>) -> bool {
        self.name() == other.name()
            && self.view().name() == other.view().name()
            && other.view().namespace() == self.view().namespace()
    }

    /// Traverse to the column's table.
    pub fn view(self) -> ViewWalker<'a> {
        self.walk(self.get().0)
    }
}
