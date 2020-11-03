pub(crate) mod expanded_alter_column;

use crate::{pair::Pair, sql_schema_differ::ColumnChanges};
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

impl SqlMigration {
    pub(crate) fn schemas(&self) -> Pair<&SqlSchema> {
        Pair::new(&self.before, &self.after)
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
    pub table_index: usize,
}

#[derive(Debug)]
pub(crate) struct DropTable {
    pub table_index: usize,
}

#[derive(Debug)]
pub(crate) struct AlterTable {
    /// Index in (previous_schema, next_schema).
    pub table_index: Pair<usize>,
    pub changes: Vec<TableChange>,
}

#[derive(Debug)]
pub(crate) enum TableChange {
    AddColumn(AddColumn),
    AlterColumn(AlterColumn),
    DropColumn(DropColumn),
    DropAndRecreateColumn {
        /// The index of the column in the table.
        column_index: Pair<usize>,
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
    pub column_index: usize,
}

#[derive(Debug)]
pub(crate) struct DropColumn {
    pub index: usize,
}

#[derive(Debug)]
pub(crate) struct AlterColumn {
    pub column_index: Pair<usize>,
    pub changes: ColumnChanges,
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
pub(crate) struct DropForeignKey {
    pub table: String,
    pub table_index: usize,
    pub foreign_key_index: usize,
    pub constraint_name: String,
}

#[derive(Debug)]
pub(crate) struct CreateIndex {
    pub table: String,
    pub index: Index,
    pub caused_by_create_table: bool,
}

#[derive(Debug)]
pub(crate) struct DropIndex {
    pub table: String,
    pub name: String,
}

#[derive(Debug)]
pub(crate) struct AlterIndex {
    pub table: String,
    pub index_name: String,
    pub index_new_name: String,
}

#[derive(Debug)]
pub(crate) struct CreateEnum {
    pub enum_index: usize,
}

#[derive(Debug)]
pub(crate) struct DropEnum {
    pub enum_index: usize,
}

#[derive(Debug)]
pub(crate) struct AlterEnum {
    pub index: Pair<usize>,
    pub created_variants: Vec<String>,
    pub dropped_variants: Vec<String>,
}

impl AlterEnum {
    pub(crate) fn is_empty(&self) -> bool {
        self.created_variants.is_empty() && self.dropped_variants.is_empty()
    }
}

#[derive(Debug)]
pub(crate) struct RedefineTable {
    pub added_columns: Vec<usize>,
    pub dropped_columns: Vec<usize>,
    pub dropped_primary_key: bool,
    pub column_pairs: Vec<(Pair<usize>, ColumnChanges, Option<ColumnTypeChange>)>,
    pub table_index: Pair<usize>,
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
                SqlMigrationStep::DropTable(DropTable { table_index: 9 }),
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
