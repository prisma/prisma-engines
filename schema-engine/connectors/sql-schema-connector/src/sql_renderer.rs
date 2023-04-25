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
#[cfg(feature = "mssql")]
mod mssql_renderer;
#[cfg(feature = "mysql")]
mod mysql_renderer;
#[cfg(feature = "postgresql")]
mod postgres_renderer;
#[cfg(feature = "sqlite")]
mod sqlite_renderer;

pub(crate) use common::IteratorJoin;

use self::common::{Quoted, QuotedWithPrefix};
use crate::{
    migration_pair::MigrationPair,
    sql_migration::{self, AlterEnum, AlterTable, RedefineTable, SequenceChanges},
};
use sql_schema_describer::{
    self as sql,
    walkers::{EnumWalker, ForeignKeyWalker, IndexWalker, TableWalker, UserDefinedTypeWalker, ViewWalker},
    SqlSchema,
};

pub(crate) trait SqlRenderer {
    fn quote<'a>(&self, name: &'a str) -> Quoted<&'a str>;

    fn render_add_foreign_key(&self, foreign_key: ForeignKeyWalker<'_>) -> String;

    fn render_alter_enum(&self, alter_enum: &AlterEnum, schemas: MigrationPair<&SqlSchema>) -> Vec<String>;

    fn render_alter_primary_key(&self, _tables: MigrationPair<TableWalker<'_>>) -> Vec<String> {
        unreachable!("unreachable render_alter_primary_key()")
    }

    fn render_alter_sequence(
        &self,
        _idx: MigrationPair<u32>,
        _: SequenceChanges,
        _: MigrationPair<&SqlSchema>,
    ) -> Vec<String> {
        unreachable!("unreachable render_alter_sequence");
    }

    fn render_rename_index(&self, _indexes: MigrationPair<IndexWalker<'_>>) -> Vec<String> {
        unreachable!("unreachable render_alter_index")
    }

    fn render_rename_primary_key(&self, _tables: MigrationPair<TableWalker<'_>>) -> Vec<String> {
        unreachable!("unreachable render_rename_index")
    }

    fn render_alter_table(&self, alter_table: &AlterTable, schemas: MigrationPair<&SqlSchema>) -> Vec<String>;

    /// Render a `CreateEnum` step.
    fn render_create_enum(&self, create_enum: EnumWalker<'_>) -> Vec<String>;

    fn render_create_index(&self, index: IndexWalker<'_>) -> String;

    /// Render a table creation step.
    fn render_create_table(&self, table: TableWalker<'_>) -> String;

    /// Render a table creation with the provided table name.
    fn render_create_table_as(&self, table: TableWalker<'_>, table_name: QuotedWithPrefix<&str>) -> String;

    fn render_drop_and_recreate_index(&self, _indexes: MigrationPair<IndexWalker<'_>>) -> Vec<String> {
        unreachable!("unreachable render_drop_and_recreate_index")
    }

    /// Render a `DropEnum` step.
    fn render_drop_enum(&self, dropped_enum: EnumWalker<'_>) -> Vec<String>;

    /// Render a `DropForeignKey` step.
    fn render_drop_foreign_key(&self, foreign_key: ForeignKeyWalker<'_>) -> String;

    /// Render a `DropIndex` step.
    fn render_drop_index(&self, index: IndexWalker<'_>) -> String;

    /// Render a `DropTable` step.
    fn render_drop_table(&self, namespace: Option<&str>, table_name: &str) -> Vec<String> {
        let name = match namespace {
            Some(namespace) => format!("{}.{}", self.quote(namespace), self.quote(table_name)),
            None => format!("{}", self.quote(table_name)),
        };
        vec![format!("DROP TABLE {name}")]
    }

    /// Render a `RedefineTables` step.
    fn render_redefine_tables(&self, tables: &[RedefineTable], schemas: MigrationPair<&SqlSchema>) -> Vec<String>;

    /// Render a table renaming step.
    fn render_rename_table(&self, namespace: Option<&str>, name: &str, new_name: &str) -> String;

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
    fn render_rename_foreign_key(&self, fks: MigrationPair<ForeignKeyWalker<'_>>) -> String;

    fn render_create_namespace(&self, _namespace: sql::NamespaceWalker<'_>) -> String {
        unreachable!()
    }

    #[cfg(feature = "postgresql")]
    fn render_create_extension(&self, _create: &sql_migration::CreateExtension, _schema: &SqlSchema) -> Vec<String> {
        unreachable!("render_create_extension")
    }

    #[cfg(feature = "postgresql")]
    fn render_alter_extension(
        &self,
        _alter: &sql_migration::AlterExtension,
        _schemas: MigrationPair<&SqlSchema>,
    ) -> Vec<String> {
        unreachable!("render_alter_extension")
    }

    #[cfg(feature = "postgresql")]
    fn render_drop_extension(&self, _drop: &sql_migration::DropExtension, _schema: &SqlSchema) -> Vec<String> {
        unreachable!("render_drop_extension")
    }
}
