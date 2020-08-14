pub(crate) mod rendered_step;

mod common;
mod mysql_renderer;
mod postgres_renderer;
mod sqlite_renderer;

pub(crate) use common::{IteratorJoin, Quoted, QuotedWithSchema};

use crate::{
    database_info::DatabaseInfo,
    sql_schema_differ::{ColumnDiffer, SqlSchemaDiffer},
    CreateEnum, DropEnum,
};
use sql_schema_describer::walkers::{ColumnWalker, TableWalker};
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

    fn render_column(&self, schema_name: &str, column: ColumnWalker<'_>, add_fk_prefix: bool) -> String;

    fn render_references(&self, schema_name: &str, foreign_key: &ForeignKey) -> String;

    fn render_default<'a>(&self, default: &'a DefaultValue, family: &ColumnTypeFamily) -> Cow<'a, str>;

    /// Attempt to render a database-specific ALTER COLUMN based on the
    /// passed-in differ. `None` means that we could not generate a good (set
    /// of) ALTER COLUMN(s), and we should fall back to dropping and recreating
    /// the column.
    fn render_alter_column(&self, differ: &ColumnDiffer<'_>) -> Option<RenderedAlterColumn>;

    /// Render a `CreateEnum` step.
    fn render_create_enum(&self, create_enum: &CreateEnum) -> Vec<String>;

    /// Render a `CreateTable` step.
    fn render_create_table(&self, table: &TableWalker<'_>, schema_name: &str) -> anyhow::Result<String>;

    /// Render a `DropEnum` step.
    fn render_drop_enum(&self, drop_enum: &DropEnum) -> Vec<String>;

    /// Render a `RedefineTables` step.
    fn render_redefine_tables(
        &self,
        tables: &[String],
        differ: SqlSchemaDiffer<'_>,
        database_info: &DatabaseInfo,
    ) -> Vec<String>;
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
