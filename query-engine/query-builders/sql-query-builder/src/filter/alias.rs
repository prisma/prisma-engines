use crate::{model_extensions::AsColumn, *};

use quaint::prelude::Column;
use query_structure::ScalarField;

#[derive(Clone, Copy, Debug)]
/// A distinction in aliasing to separate the parent table and the joined data
/// in the statement.
#[derive(Default)]
pub enum AliasMode {
    #[default]
    Table,
    Join,
}

#[derive(Clone, Copy, Debug, Default)]
/// Aliasing tool to count the nesting level to help with heavily nested
/// self-related queries.
pub struct Alias {
    counter: usize,
    mode: AliasMode,
}

impl Alias {
    /// Increment the alias as a new copy.
    ///
    /// Use when nesting one level down to a new subquery. `AliasMode` is
    /// required due to the fact the current mode can be in `AliasMode::Join`.
    pub fn inc(&self, mode: AliasMode) -> Self {
        Self {
            counter: self.counter + 1,
            mode,
        }
    }

    /// Flip the alias to a different mode keeping the same nesting count.
    pub fn flip(&self, mode: AliasMode) -> Self {
        Self {
            counter: self.counter,
            mode,
        }
    }

    /// A string representation of the current alias. The current mode can be
    /// overridden by defining the `mode_override`.
    pub fn to_string(self, mode_override: Option<AliasMode>) -> String {
        match mode_override.unwrap_or(self.mode) {
            AliasMode::Table => format!("t{}", self.counter),
            AliasMode::Join => format!("j{}", self.counter),
        }
    }

    #[cfg(feature = "relation_joins")]
    pub fn to_table_string(self) -> String {
        self.to_string(Some(AliasMode::Table))
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
            Some(alias) => self.table(alias.to_string(None)),
            None => self,
        }
    }
}
