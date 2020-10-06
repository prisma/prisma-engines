#![deny(missing_docs)]

mod common;
mod mssql_renderer;
mod mysql_renderer;
mod postgres_renderer;
mod sqlite_renderer;

pub(crate) use common::IteratorJoin;

use crate::{
    database_info::DatabaseInfo,
    sql_migration::{
        AddForeignKey, AlterEnum, AlterIndex, AlterTable, CreateEnum, CreateIndex, DropEnum, DropForeignKey, DropIndex,
    },
    sql_schema_differ::SqlSchemaDiffer,
};
use common::{Quoted, QuotedWithSchema};
use sql_schema_describer::{
    walkers::{ColumnWalker, TableWalker},
    ColumnTypeFamily, DefaultValue, ForeignKey, SqlSchema,
};
use std::borrow::Cow;

pub(crate) trait SqlRenderer {
    fn quote<'a>(&self, name: &'a str) -> Quoted<&'a str>;

    fn render_add_foreign_key(&self, add_foreign_key: &AddForeignKey) -> String;

    fn render_alter_enum(&self, alter_enum: &AlterEnum, differ: &SqlSchemaDiffer<'_>) -> Vec<String>;

    fn render_column(&self, column: ColumnWalker<'_>) -> String;

    fn render_references(&self, table: &str, foreign_key: &ForeignKey) -> String;

    fn render_default<'a>(&self, default: &'a DefaultValue, family: &ColumnTypeFamily) -> Cow<'a, str>;

    /// Render an `AlterIndex` step.
    fn render_alter_index(
        &self,
        alter_index: &AlterIndex,
        database_info: &DatabaseInfo,
        current_schema: &SqlSchema,
    ) -> Vec<String>;

    fn render_alter_table(&self, alter_table: &AlterTable, differ: &SqlSchemaDiffer<'_>) -> Vec<String>;

    /// Render a `CreateEnum` step.
    fn render_create_enum(&self, create_enum: &CreateEnum) -> Vec<String>;

    /// Render a `CreateIndex` step.
    fn render_create_index(&self, create_index: &CreateIndex) -> String;

    /// Render a `CreateTable` step.
    fn render_create_table(&self, table: &TableWalker<'_>) -> String;

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

    /// Render a table renaming step.
    fn render_rename_table(&self, name: &str, new_name: &str) -> String;
}
