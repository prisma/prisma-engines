pub(crate) mod expanded_alter_column;

use migration_connector::DatabaseMigrationMarker;
use serde::{Deserialize, Serialize};
use sql_schema_describer::{Column, ForeignKey, Index, SqlSchema, Table};

use crate::sql_schema_differ::ColumnChanges;

#[derive(Debug, Serialize, Deserialize)]
pub struct SqlMigration {
    pub before: SqlSchema,
    pub after: SqlSchema,
    pub steps: Vec<SqlMigrationStep>,
}

impl SqlMigration {
    pub fn empty() -> SqlMigration {
        SqlMigration {
            before: SqlSchema::empty(),
            after: SqlSchema::empty(),
            steps: Vec::new(),
        }
    }
}

impl DatabaseMigrationMarker for SqlMigration {
    const FILE_EXTENSION: &'static str = "sql";

    fn serialize(&self) -> serde_json::Value {
        serde_json::to_value(self).unwrap()
    }

    fn is_empty(&self) -> bool {
        self.steps.is_empty()
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub enum SqlMigrationStep {
    AddForeignKey(AddForeignKey),
    CreateTable(CreateTable),
    AlterTable(AlterTable),
    DropForeignKey(DropForeignKey),
    DropTable(DropTable),
    RenameTable { name: String, new_name: String },
    RedefineTables { names: Vec<String> },
    CreateIndex(CreateIndex),
    DropIndex(DropIndex),
    AlterIndex(AlterIndex),
    CreateEnum(CreateEnum),
    DropEnum(DropEnum),
    AlterEnum(AlterEnum),
}

impl SqlMigrationStep {
    pub(crate) fn description(&self) -> &str {
        match self {
            SqlMigrationStep::AddForeignKey(_) => "AddForeignKey",
            SqlMigrationStep::CreateTable(_) => "CreateTable",
            SqlMigrationStep::AlterTable(_) => "AlterTable",
            SqlMigrationStep::DropForeignKey(_) => "DropForeignKey",
            SqlMigrationStep::DropTable(_) => "DropTable",
            SqlMigrationStep::RenameTable { .. } => "RenameTable",
            SqlMigrationStep::RedefineTables { .. } => "RedefineTables",
            SqlMigrationStep::CreateIndex(_) => "CreateIndex",
            SqlMigrationStep::DropIndex(_) => "DropIndex",
            SqlMigrationStep::AlterIndex(_) => "AlterIndex",
            SqlMigrationStep::CreateEnum(_) => "CreateEnum",
            SqlMigrationStep::DropEnum(_) => "DropEnum",
            SqlMigrationStep::AlterEnum(_) => "AlterEnum",
        }
    }
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
pub struct AlterTable {
    pub table: Table,
    pub changes: Vec<TableChange>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub enum TableChange {
    AddColumn(AddColumn),
    AlterColumn(AlterColumn),
    DropColumn(DropColumn),
    DropAndRecreateColumn {
        column_name: String,
        /// The index of the column in the table in (previous schema, next schema).
        #[serde(skip)]
        column_index: (usize, usize),
        /// The change mask for the column.
        #[serde(skip)]
        changes: ColumnChanges,
    },
    DropPrimaryKey {
        constraint_name: Option<String>,
    },
    AddPrimaryKey {
        columns: Vec<String>,
    },
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
    pub column_name: String,
    #[serde(skip)]
    pub(crate) changes: ColumnChanges,
    #[serde(skip)]
    pub type_change: Option<ColumnTypeChange>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub enum ColumnTypeChange {
    RiskyCast,
    SafeCast,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct AddForeignKey {
    pub table: String,
    /// The index of the table in the next schema.
    #[serde(skip)]
    pub table_index: usize,
    /// The index of the foreign key in the table.
    #[serde(skip)]
    pub foreign_key_index: usize,
    pub foreign_key: ForeignKey,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct DropForeignKey {
    pub table: String,
    pub constraint_name: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct CreateIndex {
    pub table: String,
    pub index: Index,
    pub caused_by_create_table: bool,
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

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct RedefineTable {
    pub name: String,
}
