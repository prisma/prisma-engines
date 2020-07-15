pub(crate) mod rendered_step;

mod common;
mod mysql_renderer;
mod postgres_renderer;
mod sqlite_renderer;

pub(crate) use common::{IteratorJoin, Quoted, QuotedWithSchema};

use crate::{sql_schema_differ::ColumnDiffer, sql_schema_helpers::ColumnRef};
use sql_schema_describer::*;
use std::borrow::Cow;

pub(crate) trait SqlRenderer {
    fn quote<'a>(&self, name: &'a str) -> Quoted<&'a str>;

    fn quote_with_schema<'a, 'b>(&'a self, schema_name: &'a str, name: &'b str) -> QuotedWithSchema<'a, &'b str> {
        QuotedWithSchema {
            schema_name,
            name: self.quote(name),
        }
    }

    fn render_column(&self, schema_name: &str, column: ColumnRef<'_>, add_fk_prefix: bool) -> String;

    fn render_references(&self, schema_name: &str, foreign_key: &ForeignKey) -> String;

    fn render_default<'a>(&self, default: &'a DefaultValue, family: &ColumnTypeFamily) -> Cow<'a, str>;

    /// Attempt to render a database-specific ALTER COLUMN based on the
    /// passed-in differ. `None` means that we could not generate a good (set
    /// of) ALTER COLUMN(s), and we should fall back to dropping and recreating
    /// the column.
    ///
    /// The return type should be interpreted as (alter table lines, (statement before, statement after))
    fn render_alter_column(&self, differ: &ColumnDiffer<'_>) -> Option<RenderedAlterColumn>;
}

#[derive(Default)]
pub(crate) struct RenderedAlterColumn {
    /// The statements that will be included in the ALTER TABLE
    pub(crate) alter_columns: Vec<String>,
    /// The statements to be run before the ALTER TABLE.
    pub(crate) before: Option<String>,
    /// The statements to be run after the ALTER TABLE.
    pub(crate) after: Option<String>,
}
