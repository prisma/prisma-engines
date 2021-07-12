mod column;
mod differ_database;
mod enums;
mod index;
mod sql_schema_differ_flavour;
mod table;

pub(crate) use column::{ColumnChange, ColumnChanges};
pub(crate) use sql_schema_differ_flavour::SqlSchemaDifferFlavour;

use self::differ_database::DifferDatabase;
use crate::{
    pair::Pair,
    sql_migration::{self, AlterColumn, AlterTable, RedefineTable, SqlMigrationStep, TableChange},
    SqlFlavour, SqlSchema,
};
use column::ColumnTypeChange;
use datamodel::common::preview_features::PreviewFeature;
use enums::EnumDiffer;
use sql_schema_describer::{
    walkers::{EnumWalker, ForeignKeyWalker, SqlSchemaExt, TableWalker},
    ColumnId, TableId,
};
use std::collections::HashSet;
use table::TableDiffer;

pub(crate) fn calculate_steps(schemas: Pair<&SqlSchema>, flavour: &dyn SqlFlavour) -> Vec<SqlMigrationStep> {
    let differ = SqlSchemaDiffer::new(schemas, flavour);
    let mut steps: Vec<SqlMigrationStep> = Vec::new();

    differ.push_created_tables(&mut steps);
    differ.push_dropped_tables(&mut steps);
    differ.drop_indexes(&mut steps);
    differ.push_create_indexes(&mut steps);
    differ.push_altered_tables(&mut steps);
    flavour.push_enum_steps(&differ, &mut steps);
    differ.push_redefine_tables(&mut steps);

    steps.sort();

    steps
}

pub(crate) struct SqlSchemaDiffer<'a> {
    schemas: Pair<&'a SqlSchema>,
    db: DifferDatabase<'a>,
    pub(super) tables_to_redefine: HashSet<String>,
}

impl<'schema> SqlSchemaDiffer<'schema> {
    fn new(schemas: Pair<&'schema SqlSchema>, flavour: &'schema dyn SqlFlavour) -> Self {
        let db = DifferDatabase::new(schemas, flavour);
        let tables_to_redefine = HashSet::new();

        let mut differ = Self {
            schemas,
            db,
            tables_to_redefine,
        };

        differ.tables_to_redefine = std::mem::take(&mut flavour.tables_to_redefine(&differ));

        differ
    }

    fn push_created_tables(&self, steps: &mut Vec<SqlMigrationStep>) {
        for table in self.created_tables() {
            steps.push(SqlMigrationStep::CreateTable {
                table_id: table.table_id(),
            });

            if self.db.flavour.should_push_foreign_keys_from_created_tables() {
                for fk in table.foreign_keys() {
                    steps.push(SqlMigrationStep::AddForeignKey {
                        table_id: table.table_id(),
                        foreign_key_index: fk.foreign_key_index(),
                    });
                }
            }

            if self.db.flavour.should_create_indexes_from_created_tables() {
                let create_indexes_from_created_tables = table
                    .indexes()
                    .filter(|index| !self.db.flavour.should_skip_index_for_new_table(index))
                    .map(|index| SqlMigrationStep::CreateIndex {
                        table_id: (None, index.table().table_id()),
                        index_index: index.index(),
                    });

                steps.extend(create_indexes_from_created_tables);
            }
        }
    }

    // We drop the foreign keys of dropped tables first, so we can drop tables in whatever order we
    // please later.
    fn push_dropped_tables(&self, steps: &mut Vec<SqlMigrationStep>) {
        for dropped_table in self.dropped_tables() {
            steps.push(SqlMigrationStep::DropTable {
                table_id: dropped_table.table_id(),
            });

            if !self.db.flavour.should_drop_foreign_keys_from_dropped_tables() {
                continue;
            }

            for fk in dropped_table.foreign_keys() {
                steps.push(SqlMigrationStep::DropForeignKey {
                    table_id: dropped_table.table_id(),
                    foreign_key_index: fk.foreign_key_index(),
                });
            }
        }
    }

    fn push_altered_tables(&self, steps: &mut Vec<SqlMigrationStep>) {
        let tables = self
            .table_pairs()
            .filter(move |tables| !self.tables_to_redefine.contains(tables.next().name()));

        for table in tables {
            // Foreign keys
            for created_fk in table.created_foreign_keys() {
                steps.push(SqlMigrationStep::AddForeignKey {
                    table_id: created_fk.table().table_id(),
                    foreign_key_index: created_fk.foreign_key_index(),
                })
            }

            for dropped_fk in table.dropped_foreign_keys() {
                steps.push(SqlMigrationStep::DropForeignKey {
                    table_id: table.previous().table_id(),
                    foreign_key_index: dropped_fk.foreign_key_index(),
                })
            }

            // Indexes
            for i in table
                .index_pairs()
                .filter(|pair| self.db.flavour.index_should_be_renamed(pair))
            {
                let table: Pair<TableId> = table.tables.map(|t| t.table_id());
                let index: Pair<usize> = i.map(|i| i.index());

                let step = if self.db.flavour.can_alter_index() {
                    SqlMigrationStep::AlterIndex { table, index }
                } else {
                    SqlMigrationStep::RedefineIndex { table, index }
                };

                steps.push(step);
            }

            // Order matters.
            let changes: Vec<TableChange> = SqlSchemaDiffer::drop_primary_key(&table)
                .into_iter()
                .chain(SqlSchemaDiffer::drop_columns(&table))
                .chain(SqlSchemaDiffer::add_columns(&table))
                .chain(SqlSchemaDiffer::alter_columns(&table).into_iter())
                .chain(SqlSchemaDiffer::add_primary_key(&table))
                .collect();

            if changes.is_empty() {
                continue;
            }

            for column in table.column_pairs() {
                let ids = column.map(|c| c.column_id());
                self.db.flavour.push_index_changes_for_column_changes(
                    &table,
                    ids,
                    self.db.column_changes(table.tables.map(|t| t.table_id()), ids),
                    steps,
                );
            }

            steps.push(SqlMigrationStep::AlterTable(AlterTable {
                table_ids: table.tables.map(|t| t.table_id()),
                changes,
            }));
        }
    }

    fn drop_columns<'a>(differ: &'a TableDiffer<'schema, 'a>) -> impl Iterator<Item = TableChange> + 'a {
        differ.dropped_columns().map(|column| TableChange::DropColumn {
            column_id: column.column_id(),
        })
    }

    fn add_columns<'a>(differ: &'a TableDiffer<'schema, 'a>) -> impl Iterator<Item = TableChange> + 'a {
        differ.added_columns().map(move |column| TableChange::AddColumn {
            column_id: column.column_id(),
        })
    }

    fn alter_columns(table_differ: &TableDiffer<'_, '_>) -> Vec<TableChange> {
        let mut alter_columns: Vec<_> = table_differ
            .column_pairs()
            .filter_map(move |column_differ| {
                let changes = table_differ.db.column_changes(
                    table_differ.tables.map(|t| t.table_id()),
                    column_differ.map(|col| col.column_id()),
                );

                if !changes.differs_in_something() {
                    return None;
                }

                let column_id = Pair::new(column_differ.previous.column_id(), column_differ.next.column_id());

                match changes.type_change {
                    Some(ColumnTypeChange::NotCastable) => {
                        Some(TableChange::DropAndRecreateColumn { column_id, changes })
                    }
                    Some(ColumnTypeChange::RiskyCast) => Some(TableChange::AlterColumn(AlterColumn {
                        column_id,
                        changes,
                        type_change: Some(crate::sql_migration::ColumnTypeChange::RiskyCast),
                    })),
                    Some(ColumnTypeChange::SafeCast) => Some(TableChange::AlterColumn(AlterColumn {
                        column_id,
                        changes,
                        type_change: Some(crate::sql_migration::ColumnTypeChange::SafeCast),
                    })),
                    None => Some(TableChange::AlterColumn(AlterColumn {
                        column_id,
                        changes,
                        type_change: None,
                    })),
                }
            })
            .collect();

        alter_columns.sort_by_key(|alter_col| match alter_col {
            TableChange::AlterColumn(alter_col) => alter_col.column_id,
            TableChange::DropAndRecreateColumn { column_id, .. } => *column_id,
            _ => unreachable!(),
        });

        alter_columns
    }

    fn add_primary_key(differ: &TableDiffer<'_, '_>) -> Option<TableChange> {
        let from_psl_change = differ
            .created_primary_key()
            .filter(|pk| !pk.columns.is_empty())
            .map(|_| TableChange::AddPrimaryKey);

        if differ.db.flavour.should_recreate_the_primary_key_on_column_recreate() {
            from_psl_change.or_else(|| {
                let from_recreate = Self::alter_columns(differ).into_iter().any(|tc| match tc {
                    TableChange::DropAndRecreateColumn { column_id, .. } => {
                        let idx = *column_id.previous();
                        differ.previous().column_at(idx).is_part_of_primary_key()
                    }
                    _ => false,
                });

                if from_recreate {
                    Some(TableChange::AddPrimaryKey)
                } else {
                    None
                }
            })
        } else {
            from_psl_change
        }
    }

    fn drop_primary_key(differ: &TableDiffer<'_, '_>) -> Option<TableChange> {
        let from_psl_change = differ.dropped_primary_key().map(|_pk| TableChange::DropPrimaryKey);

        if differ.db.flavour.should_recreate_the_primary_key_on_column_recreate() {
            from_psl_change.or_else(|| {
                let from_recreate = Self::alter_columns(differ).into_iter().any(|tc| match tc {
                    TableChange::DropAndRecreateColumn { column_id, .. } => {
                        let idx = *column_id.previous();
                        differ.previous().column_at(idx).is_part_of_primary_key()
                    }
                    _ => false,
                });

                if from_recreate {
                    Some(TableChange::DropPrimaryKey)
                } else {
                    None
                }
            })
        } else {
            from_psl_change
        }
    }

    fn push_create_indexes(&self, steps: &mut Vec<SqlMigrationStep>) {
        for tables in self
            .table_pairs()
            .filter(|tables| !self.tables_to_redefine.contains(tables.next().name()))
        {
            for index in tables.created_indexes() {
                steps.push(SqlMigrationStep::CreateIndex {
                    table_id: (Some(tables.previous().table_id()), tables.next().table_id()),
                    index_index: index.index(),
                })
            }

            if self.db.flavour.indexes_should_be_recreated_after_column_drop() {
                let dropped_and_recreated_column_ids_next: HashSet<ColumnId> = tables
                    .column_pairs()
                    .filter(|columns| {
                        matches!(
                            self.db.column_changes_for_walkers(*columns).type_change,
                            Some(ColumnTypeChange::NotCastable)
                        )
                    })
                    .map(|col| col.next.column_id())
                    .collect();

                for index in tables.index_pairs().filter(|index| {
                    index
                        .next()
                        .columns()
                        .any(|col| dropped_and_recreated_column_ids_next.contains(&col.column_id()))
                }) {
                    steps.push(SqlMigrationStep::CreateIndex {
                        table_id: (Some(tables.previous().table_id()), tables.next().table_id()),
                        index_index: index.next().index(),
                    })
                }
            }
        }
    }

    fn drop_indexes(&self, steps: &mut Vec<SqlMigrationStep>) {
        let mut drop_indexes = HashSet::new();

        for tables in self.table_pairs() {
            for index in tables.dropped_indexes() {
                // On MySQL, foreign keys automatically create indexes. These foreign-key-created
                // indexes should only be dropped as part of the foreign key.
                if self.db.flavour.should_skip_fk_indexes() && index::index_covers_fk(tables.previous(), &index) {
                    continue;
                }

                drop_indexes.insert((index.table().table_id(), index.index()));
            }
        }

        // On SQLite, we will recreate indexes in the RedefineTables step,
        // because they are needed for implementing new foreign key constraints.
        if !self.tables_to_redefine.is_empty() && self.db.flavour.should_drop_indexes_from_dropped_tables() {
            for table in self.dropped_tables() {
                for index in table.indexes() {
                    drop_indexes.insert((index.table().table_id(), index.index()));
                }
            }
        }

        for (table_id, index_index) in drop_indexes.into_iter() {
            steps.push(SqlMigrationStep::DropIndex { table_id, index_index })
        }
    }

    fn push_redefine_tables(&self, steps: &mut Vec<SqlMigrationStep>) {
        if self.tables_to_redefine.is_empty() {
            return;
        }

        let tables_to_redefine = self
            .table_pairs()
            .filter(|tables| self.tables_to_redefine.contains(tables.next().name()))
            .map(|differ| {
                let column_pairs = differ
                    .column_pairs()
                    .map(|columns| {
                        let changes = self.db.column_changes_for_walkers(columns);
                        (
                            Pair::new(columns.previous.column_id(), columns.next.column_id()),
                            changes,
                            changes.type_change.map(|tc| match tc {
                                ColumnTypeChange::SafeCast => sql_migration::ColumnTypeChange::SafeCast,
                                ColumnTypeChange::RiskyCast => sql_migration::ColumnTypeChange::RiskyCast,
                                ColumnTypeChange::NotCastable => sql_migration::ColumnTypeChange::NotCastable,
                            }),
                        )
                    })
                    .collect();

                RedefineTable {
                    table_ids: differ.tables.as_ref().map(|t| t.table_id()),
                    dropped_primary_key: SqlSchemaDiffer::drop_primary_key(&differ).is_some(),
                    added_columns: differ.added_columns().map(|col| col.column_id()).collect(),
                    dropped_columns: differ.dropped_columns().map(|col| col.column_id()).collect(),
                    column_pairs,
                }
            })
            .collect();

        steps.push(SqlMigrationStep::RedefineTables(tables_to_redefine))
    }

    /// An iterator over the tables that are present in both schemas.
    fn table_pairs(&self) -> impl Iterator<Item = TableDiffer<'schema, '_>> + '_ {
        self.db.table_pairs().map(move |tables| TableDiffer {
            tables: self.schemas.tables(&tables),
            db: &self.db,
        })
    }

    fn created_tables(&self) -> impl Iterator<Item = TableWalker<'schema>> + '_ {
        self.db
            .created_tables()
            .map(move |table_id| self.schemas.next().table_walker_at(table_id))
    }

    fn dropped_tables(&self) -> impl Iterator<Item = TableWalker<'schema>> + '_ {
        self.db
            .dropped_tables()
            .map(move |table_id| self.schemas.previous().table_walker_at(table_id))
    }

    fn enum_pairs(&self) -> impl Iterator<Item = EnumDiffer<'_>> {
        self.previous_enums().filter_map(move |previous| {
            self.next_enums()
                .find(|next| enums_match(&previous, next))
                .map(|next| EnumDiffer {
                    enums: Pair::new(previous, next),
                })
        })
    }

    fn created_enums<'a>(&'a self) -> impl Iterator<Item = EnumWalker<'schema>> + 'a {
        self.next_enums()
            .filter(move |next| !self.previous_enums().any(|previous| enums_match(&previous, next)))
    }

    fn dropped_enums<'a>(&'a self) -> impl Iterator<Item = EnumWalker<'schema>> + 'a {
        self.previous_enums()
            .filter(move |previous| !self.next_enums().any(|next| enums_match(previous, &next)))
    }

    fn previous_enums(&self) -> impl Iterator<Item = EnumWalker<'schema>> {
        self.schemas.previous().enum_walkers()
    }

    fn next_enums(&self) -> impl Iterator<Item = EnumWalker<'schema>> {
        self.schemas.next().enum_walkers()
    }
}

/// Compare two [ForeignKey](/sql-schema-describer/struct.ForeignKey.html)s and return whether they
/// should be considered equivalent for schema diffing purposes.
fn foreign_keys_match(fks: Pair<&ForeignKeyWalker<'_>>, db: &DifferDatabase<'_>) -> bool {
    let references_same_table = db.flavour.table_names_match(fks.map(|fk| fk.referenced_table().name()));

    let references_same_column_count =
        fks.previous().referenced_columns_count() == fks.next().referenced_columns_count();

    let constrains_same_column_count =
        fks.previous().constrained_columns().count() == fks.next().constrained_columns().count();

    let constrains_same_columns = fks.interleave(|fk| fk.constrained_columns()).all(|cols| {
        let type_changed = || db.column_changes_for_walkers(cols).type_changed();

        let arities_ok = db.flavour.can_cope_with_foreign_key_column_becoming_nonnullable()
            || (cols.previous().arity() == cols.next().arity()
                || (cols.previous().arity().is_required() && cols.next().arity().is_nullable()));

        cols.previous().name() == cols.next().name() && !type_changed() && arities_ok
    });

    // Foreign key references different columns or the same columns in a different order.
    let references_same_columns = fks
        .interleave(|fk| fk.referenced_column_names())
        .all(|pair| pair.previous == pair.next);

    let matches = references_same_table
        && references_same_column_count
        && constrains_same_column_count
        && constrains_same_columns
        && references_same_columns;

    if db
        .flavour
        .preview_features()
        .contains(PreviewFeature::ReferentialActions)
    {
        let same_on_delete_action = fks.previous.on_delete_action() == fks.next.on_delete_action();
        let same_on_update_action = fks.previous.on_update_action() == fks.next.on_update_action();

        matches && same_on_delete_action && same_on_update_action
    } else {
        matches
    }
}

fn enums_match(previous: &EnumWalker<'_>, next: &EnumWalker<'_>) -> bool {
    previous.name() == next.name()
}
