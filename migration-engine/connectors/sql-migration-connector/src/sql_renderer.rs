mod common;
mod mssql_renderer;
mod mysql_renderer;
mod postgres_renderer;
mod sqlite_renderer;

pub(crate) use common::{IteratorJoin, Quoted, QuotedWithSchema};

use crate::{
    database_info::DatabaseInfo,
    sql_migration::{
        AddForeignKey, AlterEnum, AlterIndex, AlterTable, CreateEnum, CreateIndex, DropEnum, DropForeignKey, DropIndex,
    },
    sql_schema_differ::SqlSchemaDiffer,
};
use sql_schema_describer::walkers::{ColumnWalker, TableWalker};
use sql_schema_describer::*;
use std::{borrow::Cow, fmt::Write as _};

pub(crate) trait SqlRenderer {
    fn quote<'a>(&self, name: &'a str) -> Quoted<&'a str>;

    fn quote_with_schema<'a, 'b>(&'a self, name: &'b str) -> QuotedWithSchema<'a, &'b str>;

    fn render_add_foreign_key(&self, add_foreign_key: &AddForeignKey) -> String {
        let AddForeignKey { foreign_key, table } = add_foreign_key;
        let mut add_constraint = String::with_capacity(120);

        write!(
            add_constraint,
            "ALTER TABLE {table} ADD ",
            table = self.quote_with_schema(table)
        )
        .unwrap();

        if let Some(constraint_name) = foreign_key.constraint_name.as_ref() {
            write!(add_constraint, "CONSTRAINT {} ", self.quote(constraint_name)).unwrap();
        }

        write!(
            add_constraint,
            "FOREIGN KEY ({})",
            foreign_key.columns.iter().map(|col| self.quote(col)).join(", ")
        )
        .unwrap();

        add_constraint.push_str(&self.render_references(&table, &foreign_key));

        add_constraint
    }

    fn render_alter_enum(&self, alter_enum: &AlterEnum, differ: &SqlSchemaDiffer<'_>) -> anyhow::Result<Vec<String>>;

    fn render_column(&self, column: ColumnWalker<'_>) -> String;

    fn render_references(&self, table: &str, foreign_key: &ForeignKey) -> String;

    fn render_default<'a>(&self, default: &'a DefaultValue, family: &ColumnTypeFamily) -> Cow<'a, str>;

    /// Render an `AlterIndex` step.
    fn render_alter_index(
        &self,
        alter_index: &AlterIndex,
        database_info: &DatabaseInfo,
        current_schema: &SqlSchema,
    ) -> anyhow::Result<Vec<String>>;

    fn render_alter_table(&self, alter_table: &AlterTable, differ: &SqlSchemaDiffer<'_>) -> Vec<String>;

    /// Render a `CreateEnum` step.
    fn render_create_enum(&self, create_enum: &CreateEnum) -> Vec<String>;

    /// Render a `CreateIndex` step.
    fn render_create_index(&self, create_index: &CreateIndex) -> String;

    /// Render a `CreateTable` step.
    fn render_create_table(&self, table: &TableWalker<'_>) -> anyhow::Result<String>;

    /// Render a `DropEnum` step.
    fn render_drop_enum(&self, drop_enum: &DropEnum) -> Vec<String>;

    /// Render a `DropForeignKey` step.
    fn render_drop_foreign_key(&self, drop_foreign_key: &DropForeignKey) -> String;

    /// Render a `DropIndex` step.
    fn render_drop_index(&self, drop_index: &DropIndex) -> String;

    /// Render a `DropTable` step.
    fn render_drop_table(&self, table_name: &str) -> Vec<String> {
        vec![format!("DROP TABLE {}", self.quote(&table_name))]
    }

    /// Render a `RedefineTables` step.
    fn render_redefine_tables(&self, tables: &[String], differ: SqlSchemaDiffer<'_>) -> Vec<String>;

    fn render_rename_table(&self, name: &str, new_name: &str) -> String;
}
