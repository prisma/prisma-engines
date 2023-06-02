mod index_column;
mod table_column;
mod view_column;

pub use index_column::IndexColumnWalker;
pub use table_column::TableColumnWalker;
pub use view_column::ViewColumnWalker;

use std::any::Any;

use either::Either;

use crate::{
    Column, ColumnArity, ColumnType, ColumnTypeFamily, DefaultValueWalker, EnumWalker, TableColumnId,
    TableDefaultValueId, ViewColumnId, ViewDefaultValueId, Walker,
};

/// Traverse a column, that can be in a table or in a view.
pub type ColumnWalker<'a> = Walker<'a, Either<TableColumnId, ViewColumnId>>;

impl<'a> ColumnWalker<'a> {
    /// Refines the walker to either as a table or as a view column.
    pub fn refine(self) -> Either<TableColumnWalker<'a>, ViewColumnWalker<'a>> {
        match self.id {
            Either::Left(table_column_id) => Either::Left(self.walk(table_column_id)),
            Either::Right(view_column_id) => Either::Right(self.walk(view_column_id)),
        }
    }

    /// The nullability and arity of the column.
    pub fn arity(self) -> ColumnArity {
        self.get().tpe.arity
    }

    /// Returns whether the column has the enum default value of the given enum type.
    pub fn column_has_enum_default_value(self, enum_name: &str, value: &str) -> bool {
        self.column_type_family_as_enum().map(|enm| enm.name()) == Some(enum_name)
            && self
                .default()
                .and_then(|default| default.as_value())
                .and_then(|value| value.as_enum_value())
                == Some(value)
    }

    /// Returns whether the type of the column matches the provided enum name.
    pub fn column_type_is_enum(self, enum_name: &str) -> bool {
        self.column_type_family_as_enum()
            .map(|enm| enm.name() == enum_name)
            .unwrap_or(false)
    }

    /// The type family.
    pub fn column_type_family(self) -> &'a ColumnTypeFamily {
        &self.get().tpe.family
    }

    /// Extract an `Enum` column type family, or `None` if the family is something else.
    pub fn column_type_family_as_enum(self) -> Option<EnumWalker<'a>> {
        self.column_type_family().as_enum().map(|enum_id| self.walk(enum_id))
    }

    /// The column name.
    pub fn name(self) -> &'a str {
        &self.get().name
    }

    /// the full column type.
    pub fn column_type(self) -> &'a ColumnType {
        &self.get().tpe
    }

    /// the column native type.
    pub fn column_native_type<T: Any + 'static>(self) -> Option<&'a T> {
        self.column_type().native_type.as_ref().map(|nt| nt.downcast_ref())
    }

    /// is this column an auto-incrementing integer?
    pub fn is_autoincrement(self) -> bool {
        self.get().auto_increment
    }

    /// the default value for the column.
    pub fn default(self) -> Option<DefaultValueWalker<'a>> {
        match self.id {
            Either::Left(id) => self
                .schema
                .table_default_values
                .binary_search_by_key(&id, |(id, _)| *id)
                .ok()
                .map(|id| self.walk(Either::Left(TableDefaultValueId(id as u32)))),
            Either::Right(id) => self
                .schema
                .view_default_values
                .binary_search_by_key(&id, |(id, _)| *id)
                .ok()
                .map(|id| self.walk(Either::Right(ViewDefaultValueId(id as u32)))),
        }
    }

    /// returns whether two columns are named the same and belong to the same table.
    pub fn is_same_column(self, other: ColumnWalker<'_>) -> bool {
        match (self.refine(), other.refine()) {
            (Either::Left(this), Either::Left(other)) => this.is_same_column(other),
            (Either::Right(this), Either::Right(other)) => this.is_same_column(other),
            _ => false,
        }
    }

    /// True if the column is defined in a view.
    pub fn is_in_view(self) -> bool {
        self.id.is_right()
    }

    /// Description (comment) of the column.
    pub fn description(self) -> Option<&'a str> {
        self.get().description.as_deref()
    }

    fn get(self) -> &'a Column {
        match self.id {
            Either::Left(table_column_id) => &self.schema.table_columns[table_column_id.0 as usize].1,
            Either::Right(view_column_id) => &self.schema.view_columns[view_column_id.0 as usize].1,
        }
    }
}
