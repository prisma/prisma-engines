//! Render SQL DDL statements.
//!
//! Conventions:
//!
//! - Use 4 spaces for indentation (see common::SQL_INDENTATION)
//! - SQL types and keywords, like CREATE TABLE and VARCHAR, should be upper
//!   case, for consistency.
//! - SqlRenderer implementations do not add semicolons at the end of
//!   statements, this is done later.

mod common;
mod mssql_renderer;
mod mysql_renderer;
mod postgres_renderer;
mod sqlite_renderer;

pub(crate) use common::IteratorJoin;

use self::common::{Quoted, TableName};
use crate::{
    pair::Pair,
    sql_migration::{AlterEnum, AlterTable, RedefineTable, SequenceChanges},
};
use sql_schema_describer::{
    self as sql,
    walkers::{EnumWalker, ForeignKeyWalker, IndexWalker, TableWalker, UserDefinedTypeWalker, ViewWalker},
    SqlSchema,
};

pub(crate) trait SqlRenderer {
    fn quote<'a>(&self, name: &'a str) -> Quoted<&'a str>;

    fn render_add_foreign_key(&self, foreign_key: ForeignKeyWalker<'_>) -> String;

    fn render_alter_enum(&self, alter_enum: &AlterEnum, schemas: Pair<&SqlSchema>) -> Vec<String>;

    fn render_alter_primary_key(&self, _tables: Pair<TableWalker<'_>>) -> Vec<String> {
        unreachable!("unreachable render_alter_primary_key()")
    }

    fn render_alter_sequence(&self, _idx: Pair<u32>, _: SequenceChanges, _: Pair<&SqlSchema>) -> Vec<String> {
        unreachable!("unreachable render_alter_sequence");
    }

    fn render_rename_index(&self, _indexes: Pair<IndexWalker<'_>>) -> Vec<String> {
        unreachable!("unreachable render_alter_index")
    }

    fn render_rename_primary_key(&self, _tables: Pair<TableWalker<'_>>) -> Vec<String> {
        unreachable!("unreachable render_rename_index")
    }

    fn render_alter_table(&self, alter_table: &AlterTable, schemas: Pair<&SqlSchema>) -> Vec<String>;

    /// Render a `CreateEnum` step.
    fn render_create_enum(&self, create_enum: EnumWalker<'_>) -> Vec<String>;

    fn render_create_index(&self, index: IndexWalker<'_>) -> String;

    /// Render a table creation step.
    fn render_create_table(&self, table: TableWalker<'_>) -> String;

    /// Render a table creation with the provided table name.
    fn render_create_table_as(&self, table: TableWalker<'_>, table_name: TableName<&str>) -> String;

    fn render_drop_and_recreate_index(&self, _indexes: Pair<IndexWalker<'_>>) -> Vec<String> {
        unreachable!("unreachable render_drop_and_recreate_index")
    }

    /// Render a `DropEnum` step.
    fn render_drop_enum(&self, dropped_enum: EnumWalker<'_>) -> Vec<String>;

    /// Render a `DropForeignKey` step.
    fn render_drop_foreign_key(&self, foreign_key: ForeignKeyWalker<'_>) -> String;

    /// Render a `DropIndex` step.
    fn render_drop_index(&self, index: IndexWalker<'_>) -> String;

    /// Render a `DropTable` step.
    fn render_drop_table(&self, table_name: &str) -> Vec<String> {
        vec![format!("DROP TABLE {}", self.quote(table_name))]
    }

    /// Render a `RedefineTables` step.
    fn render_redefine_tables(&self, tables: &[RedefineTable], schemas: Pair<&SqlSchema>) -> Vec<String>;

    /// Render a table renaming step.
    fn render_rename_table(&self, name: &str, new_name: &str) -> String;

    /// Render a drop view step.
    fn render_drop_view(&self, view: ViewWalker<'_>) -> String;

    /// Render a drop type step.
    fn render_drop_user_defined_type(&self, udt: &UserDefinedTypeWalker<'_>) -> String;

    /// Render a transaction start.
    fn render_begin_transaction(&self) -> Option<&'static str> {
        None
    }

    /// Render a transaction commit.
    fn render_commit_transaction(&self) -> Option<&'static str> {
        None
    }

    /// Render a `RenameForeignKey` step.
    fn render_rename_foreign_key(&self, fks: Pair<ForeignKeyWalker<'_>>) -> String;

    fn render_create_namespace(&self, _namespace: sql::NamespaceWalker<'_>) -> String {
        unreachable!()
    }
}
