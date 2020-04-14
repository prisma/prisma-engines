pub(crate) mod expanded_alter_column;

use migration_connector::DatabaseMigrationMarker;
use serde::{Deserialize, Serialize};
use sql_schema_describer::{Column, ForeignKey, Index, SqlSchema, Table};

#[derive(Debug, Serialize, Deserialize)]
pub struct SqlMigration {
    pub before: SqlSchema,
    pub after: SqlSchema,
    pub original_steps: Vec<SqlMigrationStep>,
    /// The `original_steps`, but with specific corrections applied (notably for SQLite) when the
    /// original steps cannot be applied directly, e.g. because some operations are not supported
    /// by the database.
    pub corrected_steps: Vec<SqlMigrationStep>,
    pub rollback: Vec<SqlMigrationStep>,
}

impl SqlMigration {
    pub fn empty() -> SqlMigration {
        SqlMigration {
            before: SqlSchema::empty(),
            after: SqlSchema::empty(),
            original_steps: Vec::new(),
            corrected_steps: Vec::new(),
            rollback: Vec::new(),
        }
    }
}

impl DatabaseMigrationMarker for SqlMigration {
    fn serialize(&self) -> serde_json::Value {
        serde_json::to_value(self).unwrap()
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub enum SqlMigrationStep {
    AddForeignKey(AddForeignKey),
    CreateTable(CreateTable),
    AlterTable(AlterTable),
    DropTable(DropTable),
    DropTables(DropTables),
    RenameTable { name: String, new_name: String },
    RawSql { raw: String },
    CreateIndex(CreateIndex),
    DropIndex(DropIndex),
    AlterIndex(AlterIndex),
    CreateEnum(CreateEnum),
    DropEnum(DropEnum),
    AlterEnum(AlterEnum),
}

/// A helper struct to serialize an [SqlMigrationStep](/sql-migration/enum.SqlMigrationStep.html)
/// with an additional `raw` field containing the rendered SQL string for that step.
#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct PrettySqlMigrationStep {
    #[serde(flatten)]
    pub step: SqlMigrationStep,
    pub raw: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct CreateTable {
    pub table: Table,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct DropTable {
    pub name: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct DropTables {
    pub names: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct AlterTable {
    pub table: Table,
    pub changes: Vec<TableChange>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub enum TableChange {
    AddColumn(AddColumn),
    AlterColumn(AlterColumn),
    DropColumn(DropColumn),
    DropForeignKey(DropForeignKey),
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct AddColumn {
    pub column: Column,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct DropColumn {
    pub name: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct AlterColumn {
    pub name: String,
    pub column: Column,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct AddForeignKey {
    pub table: String,
    pub foreign_key: ForeignKey,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct DropForeignKey {
    pub constraint_name: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct CreateIndex {
    pub table: String,
    pub index: Index,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct DropIndex {
    pub table: String,
    pub name: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct AlterIndex {
    pub table: String,
    pub index_name: String,
    pub index_new_name: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct CreateEnum {
    pub name: String,
    pub variants: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct DropEnum {
    pub name: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct AlterEnum {
    pub name: String,
    pub created_variants: Vec<String>,
    pub dropped_variants: Vec<String>,
}

impl AlterEnum {
    pub(crate) fn is_empty(&self) -> bool {
        self.created_variants.is_empty() && self.dropped_variants.is_empty()
    }
}
