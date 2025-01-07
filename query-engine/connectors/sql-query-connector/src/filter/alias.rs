use std::fmt;

use crate::{model_extensions::AsColumn, *};

use quaint::prelude::Column;
use query_structure::ScalarField;

#[derive(Debug, Clone, Copy)]
pub enum Alias {
    Table(usize),
    Join(usize),
}

impl Alias {
    pub fn to_join_alias(self) -> Self {
        match self {
            Self::Table(index) | Self::Join(index) => Self::Join(index),
        }
    }

    pub fn to_table_alias(self) -> Self {
        match self {
            Self::Table(index) | Self::Join(index) => Self::Table(index),
        }
    }
}

impl fmt::Display for Alias {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Table(index) => write!(f, "t{}", index),
            Self::Join(index) => write!(f, "j{}", index),
        }
    }
}

pub(crate) trait AliasedColumn {
    /// Conversion to a column. Column will point to the given alias if provided, otherwise the fully qualified path.
    ///
    /// Alias should be used only when nesting, making the top level queries
    /// more explicit.
    fn aliased_col(self, alias: Option<Alias>, ctx: &Context<'_>) -> Column<'static>;
}

impl AliasedColumn for &ScalarField {
    fn aliased_col(self, alias: Option<Alias>, ctx: &Context<'_>) -> Column<'static> {
        self.as_column(ctx).aliased_col(alias, ctx)
    }
}

impl AliasedColumn for Column<'static> {
    fn aliased_col(self, alias: Option<Alias>, _ctx: &Context<'_>) -> Column<'static> {
        match alias {
            Some(alias) => self.table(alias.to_string()),
            None => self,
        }
    }
}
