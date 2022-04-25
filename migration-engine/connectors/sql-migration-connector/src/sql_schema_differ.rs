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
    database_schema::SqlDatabaseSchema,
    pair::Pair,
    sql_migration::{self, AlterColumn, AlterTable, RedefineTable, SqlMigrationStep, TableChange},
    SqlFlavour,
};
use column::ColumnTypeChange;
use sql_schema_describer::{walkers::ForeignKeyWalker, ColumnId, TableId};
use std::collections::HashSet;
use table::TableDiffer;

pub(crate) fn calculate_steps(schemas: Pair<&SqlDatabaseSchema>, flavour: &dyn SqlFlavour) -> Vec<SqlMigrationStep> {
    let db = DifferDatabase::new(schemas, flavour);
    let mut steps: Vec<SqlMigrationStep> = Vec::new();

    push_created_table_steps(&mut steps, &db);
    push_dropped_table_steps(&mut steps, &db);
    push_dropped_index_steps(&mut steps, &db);
    push_created_index_steps(&mut steps, &db);
    push_altered_table_steps(&mut steps, &db);
    flavour.push_enum_steps(&mut steps, &db);
    push_redefined_table_steps(&mut steps, &db);

    steps.sort();

    steps
}

fn push_created_table_steps(steps: &mut Vec<SqlMigrationStep>, db: &DifferDatabase<'_>) {
    for table in db.created_tables() {
        steps.push(SqlMigrationStep::CreateTable {
            table_id: table.table_id(),
        });

        if db.flavour.should_push_foreign_keys_from_created_tables() {
            for fk in table.foreign_keys() {
                steps.push(SqlMigrationStep::AddForeignKey {
                    table_id: table.table_id(),
                    foreign_key_index: fk.foreign_key_index(),
                });
            }
        }

        if db.flavour.should_create_indexes_from_created_tables() {
            let create_indexes_from_created_tables = table
                .indexes()
                .filter(|index| !db.flavour.should_skip_index_for_new_table(index))
                .map(|index| SqlMigrationStep::CreateIndex {
                    table_id: (None, index.table().table_id()),
                    index_index: index.index(),
                    from_drop_and_recreate: false,
                });

            steps.extend(create_indexes_from_created_tables);
        }
    }
}

// We drop the foreign keys of dropped tables first, so we can drop tables in whatever order we
// please later.
fn push_dropped_table_steps(steps: &mut Vec<SqlMigrationStep>, db: &DifferDatabase<'_>) {
    for dropped_table in db.dropped_tables() {
        steps.push(SqlMigrationStep::DropTable {
            table_id: dropped_table.table_id(),
        });

        if !db.flavour.should_drop_foreign_keys_from_dropped_tables() {
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

fn push_altered_table_steps(steps: &mut Vec<SqlMigrationStep>, db: &DifferDatabase<'_>) {
    for table in db.non_redefined_table_pairs() {
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

        for fk in table.foreign_key_pairs() {
            push_foreign_key_pair_changes(fk, steps, db)
        }

        push_alter_primary_key(&table, steps);

        // Indexes.
        for i in table
            .index_pairs()
            .filter(|pair| db.flavour.index_should_be_renamed(pair))
        {
            let table: Pair<TableId> = table.tables.map(|t| t.table_id());
            let index: Pair<usize> = i.map(|i| i.index());

            let step = if db.flavour.can_rename_index() {
                SqlMigrationStep::RenameIndex { table, index }
            } else {
                SqlMigrationStep::RedefineIndex { table, index }
            };

            steps.push(step);
        }

        // Order matters.
        let mut changes = Vec::new();
        if let Some(change) = dropped_primary_key(&table) {
            changes.push(change)
        }

        if let Some(change) = renamed_primary_key(&table) {
            changes.push(change);
        }

        dropped_columns(&table, &mut changes);
        added_columns(&table, &mut changes);

        for change in alter_columns(&table) {
            changes.push(change)
        }

        if let Some(change) = added_primary_key(&table) {
            changes.push(change)
        }

        if changes.is_empty() {
            continue;
        }

        for column in table.column_pairs() {
            let ids = column.map(|c| c.column_id());
            db.flavour.push_index_changes_for_column_changes(
                &table,
                ids,
                db.column_changes(table.tables.map(|t| t.table_id()), ids),
                steps,
            );
        }

        steps.push(SqlMigrationStep::AlterTable(AlterTable {
            table_ids: table.tables.map(|t| t.table_id()),
            changes,
        }));
    }
}

fn dropped_columns(differ: &TableDiffer<'_, '_>, changes: &mut Vec<TableChange>) {
    for column in differ.dropped_columns() {
        changes.push(TableChange::DropColumn {
            column_id: column.column_id(),
        })
    }
}

fn added_columns(differ: &TableDiffer<'_, '_>, changes: &mut Vec<TableChange>) {
    for column in differ.added_columns() {
        changes.push(TableChange::AddColumn {
            column_id: column.column_id(),
            has_virtual_default: next_column_has_virtual_default(
                (column.table().table_id(), column.column_id()),
                differ.db,
            ),
        })
    }
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
                Some(ColumnTypeChange::NotCastable) => Some(TableChange::DropAndRecreateColumn { column_id, changes }),
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

fn added_primary_key(differ: &TableDiffer<'_, '_>) -> Option<TableChange> {
    // ALTER PRIMARY KEY instead where possible (e.g. cockroachdb)
    if differ.tables.previous.primary_key().is_some()
        && differ.tables.next.primary_key().is_some()
        && differ.db.flavour.can_alter_primary_keys()
    {
        return None;
    }

    let from_psl_change = differ
        .created_primary_key()
        .filter(|pk| !pk.columns.is_empty())
        .map(|_| TableChange::AddPrimaryKey)
        .or_else(|| Some(TableChange::AddPrimaryKey).filter(|_| differ.primary_key_changed()));

    if differ.db.flavour.should_recreate_the_primary_key_on_column_recreate() {
        from_psl_change.or_else(|| {
            let from_recreate = alter_columns(differ).into_iter().any(|tc| match tc {
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

fn dropped_primary_key(differ: &TableDiffer<'_, '_>) -> Option<TableChange> {
    let from_psl_change = differ
        .dropped_primary_key()
        .map(|_pk| TableChange::DropPrimaryKey)
        .or_else(|| Some(TableChange::DropPrimaryKey).filter(|_| differ.primary_key_changed()));

    // ALTER PRIMARY KEY instead where possible (e.g. cockroachdb)
    if differ.tables.previous.primary_key().is_some()
        && differ.tables.next.primary_key().is_some()
        && differ.db.flavour.can_alter_primary_keys()
    {
        return None;
    }

    if differ.db.flavour.should_recreate_the_primary_key_on_column_recreate() {
        from_psl_change.or_else(|| {
            let from_recreate = alter_columns(differ).into_iter().any(|tc| match tc {
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

fn renamed_primary_key(differ: &TableDiffer<'_, '_>) -> Option<TableChange> {
    differ
        .tables
        .map(|pk| pk.primary_key().and_then(|pk| pk.constraint_name.as_ref()))
        .transpose()
        .filter(|names| names.previous != names.next)
        .map(|_| TableChange::RenamePrimaryKey)
}

fn push_alter_primary_key(differ: &TableDiffer<'_, '_>, steps: &mut Vec<SqlMigrationStep>) {
    if !differ.db.flavour.can_alter_primary_keys() {
        return;
    }

    let (previous, next) = match differ.tables.map(|t| t.primary_key()).into_tuple() {
        (Some(previous), Some(next)) => (previous, next),
        _ => return,
    };

    if previous.column_names().len() == next.column_names().len()
        && previous.column_names().zip(next.column_names()).all(|(p, n)| p == n)
    {
        return;
    }

    steps.push(SqlMigrationStep::AlterPrimaryKey(differ.table_ids()))
}

fn push_created_index_steps(steps: &mut Vec<SqlMigrationStep>, db: &DifferDatabase<'_>) {
    for tables in db.non_redefined_table_pairs() {
        for index in tables.created_indexes() {
            steps.push(SqlMigrationStep::CreateIndex {
                table_id: (Some(tables.previous().table_id()), tables.next().table_id()),
                index_index: index.index(),
                from_drop_and_recreate: false,
            })
        }

        if db.flavour.indexes_should_be_recreated_after_column_drop() {
            let dropped_and_recreated_column_ids_next: HashSet<ColumnId> = tables
                .column_pairs()
                .filter(|columns| {
                    matches!(
                        db.column_changes_for_walkers(*columns).type_change,
                        Some(ColumnTypeChange::NotCastable)
                    )
                })
                .map(|col| col.next.column_id())
                .collect();

            for index in tables.index_pairs().filter(|index| {
                index
                    .next()
                    .columns()
                    .any(|col| dropped_and_recreated_column_ids_next.contains(&col.as_column().column_id()))
            }) {
                steps.push(SqlMigrationStep::CreateIndex {
                    table_id: (Some(tables.previous().table_id()), tables.next().table_id()),
                    index_index: index.next().index(),
                    from_drop_and_recreate: true,
                })
            }
        }
    }
}

fn push_dropped_index_steps(steps: &mut Vec<SqlMigrationStep>, db: &DifferDatabase<'_>) {
    let mut drop_indexes = HashSet::new();

    for tables in db.non_redefined_table_pairs() {
        for index in tables.dropped_indexes() {
            // On MySQL, foreign keys automatically create indexes. These foreign-key-created
            // indexes should only be dropped as part of the foreign key.
            if db.flavour.should_skip_fk_indexes() && index::index_covers_fk(tables.previous(), &index) {
                continue;
            }

            drop_indexes.insert((index.table().table_id(), index.index()));
        }
    }

    // On SQLite, we will recreate indexes in the RedefineTables step,
    // because they are needed for implementing new foreign key constraints.
    if !db.tables_to_redefine.is_empty() && db.flavour.should_drop_indexes_from_dropped_tables() {
        for table in db.dropped_tables() {
            for index in table.indexes() {
                drop_indexes.insert((index.table().table_id(), index.index()));
            }
        }
    }

    for (table_id, index_index) in drop_indexes.into_iter() {
        steps.push(SqlMigrationStep::DropIndex { table_id, index_index })
    }
}

fn push_redefined_table_steps(steps: &mut Vec<SqlMigrationStep>, db: &DifferDatabase<'_>) {
    if db.tables_to_redefine.is_empty() {
        return;
    }

    let tables_to_redefine = db
        .table_pairs()
        .filter(|tables| db.tables_to_redefine.contains(&tables.table_ids()))
        .map(|differ| {
            let column_pairs = differ
                .column_pairs()
                .map(|columns| {
                    let changes = db.column_changes_for_walkers(columns);
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
                dropped_primary_key: dropped_primary_key(&differ).is_some(),
                added_columns: differ.added_columns().map(|col| col.column_id()).collect(),
                added_columns_with_virtual_defaults: differ
                    .added_columns()
                    .filter(|col| next_column_has_virtual_default((col.table().table_id(), col.column_id()), differ.db))
                    .map(|col| col.column_id())
                    .collect(),
                dropped_columns: differ.dropped_columns().map(|col| col.column_id()).collect(),
                column_pairs,
            }
        })
        .collect();

    steps.push(SqlMigrationStep::RedefineTables(tables_to_redefine))
}

/// Compare two foreign keys and return whether they should be considered
/// equivalent for schema diffing purposes.
fn foreign_keys_match(fks: Pair<&ForeignKeyWalker<'_>>, db: &DifferDatabase<'_>) -> bool {
    let references_same_table = db.flavour.table_names_match(fks.map(|fk| fk.referenced_table().name()));

    let references_same_column_count =
        fks.previous().referenced_columns_count() == fks.next().referenced_columns_count();

    let constrains_same_column_count =
        fks.previous().constrained_columns().count() == fks.next().constrained_columns().count();

    let constrains_same_columns = fks.interleave(|fk| fk.constrained_columns()).all(|cols| {
        let type_changed = || db.column_changes_for_walkers(cols).type_changed();

        let arities_ok = db.flavour.can_cope_with_foreign_key_column_becoming_non_nullable()
            || (cols.previous().arity() == cols.next().arity()
                || (cols.previous().arity().is_required() && cols.next().arity().is_nullable()));

        cols.previous().name() == cols.next().name() && !type_changed() && arities_ok
    });

    // Foreign key references different columns or the same columns in a different order.
    let references_same_columns = fks
        .interleave(|fk| fk.referenced_column_names())
        .all(|pair| pair.previous == pair.next);

    let same_on_delete_action = fks.previous.on_delete_action() == fks.next.on_delete_action();
    let same_on_update_action = fks.previous.on_update_action() == fks.next.on_update_action();

    references_same_table
        && references_same_column_count
        && constrains_same_column_count
        && constrains_same_columns
        && references_same_columns
        && same_on_delete_action
        && same_on_update_action
}

fn push_foreign_key_pair_changes(
    fk: Pair<ForeignKeyWalker<'_>>,
    steps: &mut Vec<SqlMigrationStep>,
    db: &DifferDatabase<'_>,
) {
    // Is the referenced table being redefined, meaning we need to drop and recreate
    // the foreign key?
    if db.table_is_redefined(fk.previous().referenced_table().name())
        && !db.flavour.can_redefine_tables_with_inbound_foreign_keys()
    {
        steps.push(SqlMigrationStep::DropForeignKey {
            table_id: fk.previous().table().table_id(),
            foreign_key_index: fk.previous().foreign_key_index(),
        });
        steps.push(SqlMigrationStep::AddForeignKey {
            table_id: fk.next().table().table_id(),
            foreign_key_index: fk.next().foreign_key_index(),
        });
        return;
    }

    if db.flavour.has_unnamed_foreign_keys() {
        return;
    }

    if fk
        .map(|fk| fk.constraint_name())
        .transpose()
        .map(|names| names.previous() != names.next())
        .unwrap_or(false)
    {
        // Rename the foreign key.

        // Since we are using the conventional foreign key names for the foreign keys of
        // many-to-many relation tables, but we used not to (we did not provide a constraint
        // names), and we do not want to cause new migrations on upgrade, we ignore the foreign
        // keys of implicit many-to-many relation tables for renamings.
        if fk.map(|fk| is_prisma_implicit_m2m_fk(fk)).as_tuple() == (&true, &true) {
            return;
        }

        if db.flavour.can_rename_foreign_key() {
            steps.push(SqlMigrationStep::RenameForeignKey {
                table_id: fk.map(|fk| fk.table().table_id()),
                foreign_key_id: fk.map(|fk| fk.foreign_key_index()),
            })
        } else {
            steps.push(SqlMigrationStep::AddForeignKey {
                table_id: fk.next().table().table_id(),
                foreign_key_index: fk.next().foreign_key_index(),
            });
            steps.push(SqlMigrationStep::DropForeignKey {
                table_id: fk.previous().table().table_id(),
                foreign_key_index: fk.previous().foreign_key_index(),
            })
        }
    }
}

fn next_column_has_virtual_default((table_id, column_id): (TableId, ColumnId), db: &DifferDatabase<'_>) -> bool {
    db.schemas()
        .next()
        .prisma_level_defaults
        .binary_search(&(table_id.0, column_id.0))
        .is_ok()
}

fn is_prisma_implicit_m2m_fk(fk: ForeignKeyWalker<'_>) -> bool {
    let table = fk.table();

    if table.columns().count() != 2 {
        return false;
    }

    table.column("A").is_some() && table.column("B").is_some()
}
