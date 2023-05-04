use super::{differ_database::DifferDatabase, ColumnTypeChange};
use crate::{migration_pair::MigrationPair, sql_migration::SqlMigrationStep, sql_schema_differ};
use sql_schema_describer::{
    walkers::{IndexWalker, TableColumnWalker, TableWalker},
    TableColumnId,
};

#[cfg(feature = "mssql")]
mod mssql;
#[cfg(feature = "mysql")]
mod mysql;
#[cfg(feature = "postgresql")]
mod postgres;
#[cfg(feature = "sqlite")]
mod sqlite;

/// Trait to specialize SQL schema diffing (resulting in migration steps) by SQL backend.
pub(crate) trait SqlSchemaDifferFlavour {
    fn can_alter_primary_keys(&self) -> bool {
        false
    }

    fn can_redefine_tables_with_inbound_foreign_keys(&self) -> bool {
        false
    }

    /// If this returns `true`, the differ will generate
    /// SqlMigrationStep::RedefineIndex steps instead of
    /// SqlMigrationStep::AlterIndex.
    fn can_rename_index(&self) -> bool {
        true
    }

    /// Returns true only if the database can cope with an optional column
    /// constrained by a foreign key being made NOT NULL.
    fn can_cope_with_foreign_key_column_becoming_non_nullable(&self) -> bool {
        true
    }

    /// Controls whether we will generate `RenameForeignKey` steps for this flavour.
    fn can_rename_foreign_key(&self) -> bool;

    /// This method must return whether a column became or ceased to be autoincrementing.
    fn column_autoincrement_changed(&self, columns: MigrationPair<TableColumnWalker<'_>>) -> bool {
        columns.previous.is_autoincrement() != columns.next.is_autoincrement()
    }

    /// Return whether a column's type needs to be migrated, and how.
    fn column_type_change(&self, differ: MigrationPair<TableColumnWalker<'_>>) -> Option<ColumnTypeChange>;

    /// Push enum-related steps.
    fn push_enum_steps(&self, _steps: &mut Vec<SqlMigrationStep>, _db: &DifferDatabase<'_>) {}

    /// Push AlterSequence steps.
    fn push_alter_sequence_steps(&self, _steps: &mut Vec<SqlMigrationStep>, _db: &DifferDatabase<'_>) {}

    /// Push AlterExtension steps.
    fn push_extension_steps(&self, _steps: &mut Vec<SqlMigrationStep>, _db: &DifferDatabase<'_>) {}

    /// Define database-specific extension modules.
    fn define_extensions(&self, _db: &mut DifferDatabase<'_>) {}

    /// Connector-specific criterias deciding whether two indexes match.
    fn indexes_match(&self, _a: IndexWalker<'_>, _b: IndexWalker<'_>) -> bool {
        true
    }

    /// Returns whether the underlying database implicitly drops indexes on dropped (and potentially recreated) columns.
    fn indexes_should_be_recreated_after_column_drop(&self) -> bool {
        false
    }

    /// Return whether an index should be renamed by the migration.
    fn index_should_be_renamed(&self, indexes: MigrationPair<IndexWalker<'_>>) -> bool {
        indexes.previous.name() != indexes.next.name()
    }

    fn lower_cases_table_names(&self) -> bool {
        false
    }

    /// Did something connector-specific change in the primary key definition?
    fn primary_key_changed(&self, _tables: MigrationPair<TableWalker<'_>>) -> bool {
        false
    }

    /// Evaluate indexes/constraints that need to be dropped and re-created based on other changes in the schema
    fn push_index_changes_for_column_changes(
        &self,
        _table: &sql_schema_differ::TableDiffer<'_, '_>,
        _column_index: MigrationPair<TableColumnId>,
        _column_changes: sql_schema_differ::ColumnChanges,
        _steps: &mut Vec<SqlMigrationStep>,
    ) {
    }

    /// Whether the differ should produce CreateIndex steps for the indexes of
    /// new tables.
    fn should_create_indexes_from_created_tables(&self) -> bool {
        true
    }

    /// Whether the indexes of dropped tables should be dropped before the table
    /// is dropped.
    fn should_drop_indexes_from_dropped_tables(&self) -> bool {
        false
    }

    /// Whether the foreign keys of dropped tables should be dropped before the table
    /// is dropped.
    fn should_drop_foreign_keys_from_dropped_tables(&self) -> bool {
        true
    }

    /// Whether to skip diffing JSON defaults.
    fn should_ignore_json_defaults(&self) -> bool {
        false
    }

    /// Whether `AddForeignKey` steps should be generated for created tables.
    fn should_push_foreign_keys_from_created_tables(&self) -> bool {
        true
    }

    /// Whether indexes matching a foreign key should be skipped.
    fn should_skip_fk_indexes(&self) -> bool {
        false
    }

    /// Whether a specific index should *not* be produced.
    fn should_skip_index_for_new_table(&self, _index: IndexWalker<'_>) -> bool {
        false
    }

    /// Whether the primary key should be recreated if the column part of it is
    /// recreated.
    fn should_recreate_the_primary_key_on_column_recreate(&self) -> bool {
        false
    }

    /// Does the sql expression string match the provided byte array?
    fn string_matches_bytes(&self, string: &str, bytes: &[u8]) -> bool {
        string.as_bytes() == bytes
    }

    fn table_names_match(&self, names: MigrationPair<&str>) -> bool {
        names.previous == names.next
    }

    /// Return the tables that cannot be migrated without being redefined. This
    /// is currently useful only on SQLite.
    fn set_tables_to_redefine(&self, _db: &mut DifferDatabase<'_>) {}

    /// By implementing this method, the flavour signals the differ that
    /// specific tables should be ignored. This is mostly for system tables.
    fn table_should_be_ignored(&self, _table_name: &str) -> bool {
        false
    }

    fn view_should_be_ignored(&self, _view_name: &str) -> bool {
        false
    }

    /// Supports named Foreign Keys.
    fn has_unnamed_foreign_keys(&self) -> bool {
        false
    }
}
