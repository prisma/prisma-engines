pub(crate) mod expanded_alter_column;

use crate::sql_schema_differ::ColumnChanges;
use migration_connector::DatabaseMigrationMarker;
use serde::{Serialize, Serializer};
use sql_schema_describer::{Index, SqlSchema};

/// The database migration type for SqlMigrationConnector.
#[derive(Debug, Serialize)]
pub struct SqlMigration {
    pub(crate) before: SqlSchema,
    pub(crate) after: SqlSchema,
    pub(crate) steps: Vec<SqlMigrationStep>,
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

#[derive(Debug)]
pub(crate) enum SqlMigrationStep {
    AddForeignKey(AddForeignKey),
    CreateTable(CreateTable),
    AlterTable(AlterTable),
    DropForeignKey(DropForeignKey),
    DropTable(DropTable),
    RedefineTables(Vec<RedefineTable>),
    CreateIndex(CreateIndex),
    DropIndex(DropIndex),
    AlterIndex(AlterIndex),
    CreateEnum(CreateEnum),
    DropEnum(DropEnum),
    AlterEnum(AlterEnum),
}

impl Serialize for SqlMigrationStep {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_newtype_variant(
            "SqlMigrationStep",
            0,
            self.description(),
            &serde_json::Value::Object(Default::default()),
        )
    }
}

impl SqlMigrationStep {
    pub(crate) fn description(&self) -> &'static str {
        match self {
            SqlMigrationStep::AddForeignKey(_) => "AddForeignKey",
            SqlMigrationStep::CreateTable(_) => "CreateTable",
            SqlMigrationStep::AlterTable(_) => "AlterTable",
            SqlMigrationStep::DropForeignKey(_) => "DropForeignKey",
            SqlMigrationStep::DropTable(_) => "DropTable",
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

#[derive(Debug)]
pub(crate) struct CreateTable {
    pub(crate) table_index: usize,
}

#[derive(Debug)]
pub(crate) struct DropTable {
    pub(crate) name: String,
}

#[derive(Debug)]
pub(crate) struct AlterTable {
    /// Index in (previous_schema, next_schema).
    pub(crate) table_index: (usize, usize),
    pub(crate) changes: Vec<TableChange>,
}

#[derive(Debug)]
pub(crate) enum TableChange {
    AddColumn(AddColumn),
    AlterColumn(AlterColumn),
    DropColumn(DropColumn),
    DropAndRecreateColumn {
        /// The index of the column in the table in (previous schema, next schema).
        column_index: (usize, usize),
        /// The change mask for the column.
        changes: ColumnChanges,
    },
    DropPrimaryKey {
        constraint_name: Option<String>,
    },
    AddPrimaryKey {
        columns: Vec<String>,
    },
}

#[derive(Debug)]
pub(crate) struct AddColumn {
    pub(crate) column_index: usize,
}

#[derive(Debug)]
pub(crate) struct DropColumn {
    pub name: String,
    pub(crate) index: usize,
}

#[derive(Debug)]
pub(crate) struct AlterColumn {
    pub column_index: (usize, usize),
    pub(crate) changes: ColumnChanges,
    pub type_change: Option<ColumnTypeChange>,
}

#[derive(Debug)]
pub(crate) enum ColumnTypeChange {
    RiskyCast,
    SafeCast,
}

#[derive(Debug)]
pub(crate) struct AddForeignKey {
    /// The index of the table in the next schema.
    pub(crate) table_index: usize,
    /// The index of the foreign key in the table.
    pub(crate) foreign_key_index: usize,
}

#[derive(Debug)]
pub struct DropForeignKey {
    pub(crate) table: String,
    pub(crate) table_index: usize,
    pub(crate) foreign_key_index: usize,
    pub(crate) constraint_name: String,
}

#[derive(Debug)]
pub struct CreateIndex {
    pub(crate) table: String,
    pub(crate) index: Index,
    pub(crate) caused_by_create_table: bool,
}

#[derive(Debug)]
pub struct DropIndex {
    pub(crate) table: String,
    pub(crate) name: String,
}

#[derive(Debug)]
pub struct AlterIndex {
    pub(crate) table: String,
    pub(crate) index_name: String,
    pub(crate) index_new_name: String,
}

#[derive(Debug)]
pub struct CreateEnum {
    pub(crate) name: String,
    pub(crate) variants: Vec<String>,
}

#[derive(Debug)]
pub struct DropEnum {
    pub(crate) name: String,
}

#[derive(Debug)]
pub(crate) struct AlterEnum {
    pub(crate) name: String,
    pub(crate) created_variants: Vec<String>,
    pub(crate) dropped_variants: Vec<String>,
}

impl AlterEnum {
    pub(crate) fn is_empty(&self) -> bool {
        self.created_variants.is_empty() && self.dropped_variants.is_empty()
    }
}

#[derive(Debug)]
pub(crate) struct RedefineTable {
    pub(crate) added_columns: Vec<usize>,
    pub(crate) dropped_columns: Vec<usize>,
    pub(crate) dropped_primary_key: bool,
    pub(crate) column_pairs: Vec<(usize, usize, ColumnChanges, Option<ColumnTypeChange>)>,
    pub(crate) table_index: (usize, usize),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sql_migration_serializes_as_expected() {
        let migration = SqlMigration {
            before: SqlSchema::empty(),
            after: SqlSchema::empty(),
            steps: vec![
                SqlMigrationStep::AddForeignKey(AddForeignKey {
                    table_index: 0,
                    foreign_key_index: 0,
                }),
                SqlMigrationStep::RedefineTables(vec![]),
                SqlMigrationStep::DropTable(DropTable { name: "myTable".into() }),
            ],
        };

        let expected = serde_json::json!({
            "before": {
                "tables": [],
                "enums": [],
                "sequences": [],
            },
            "after": {
                "tables": [],
                "enums": [],
                "sequences": [],
            },
            "steps": [
                { "AddForeignKey": {} },
                { "RedefineTables": {} },
                { "DropTable": {} },
            ],
        });

        let actual = serde_json::to_value(migration).unwrap();

        assert_eq!(actual, expected);
    }
}
