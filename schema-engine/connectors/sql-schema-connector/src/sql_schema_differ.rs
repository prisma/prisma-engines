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
    migration_pair::MigrationPair,
    sql_migration::{self, AlterColumn, AlterTable, RedefineTable, SqlMigrationStep, TableChange},
    SqlFlavour,
};
use column::ColumnTypeChange;
use sql_schema_describer::{walkers::ForeignKeyWalker, IndexId, TableColumnId, Walker};
use std::{borrow::Cow, collections::HashSet};
use table::TableDiffer;

pub(crate) fn calculate_steps(
    schemas: MigrationPair<&SqlDatabaseSchema>,
    flavour: &dyn SqlFlavour,
) -> Vec<SqlMigrationStep> {
    let db = DifferDatabase::new(schemas, flavour);
    let mut steps: Vec<SqlMigrationStep> = Vec::new();

    flavour.push_extension_steps(&mut steps, &db);

    push_created_schema_steps(&mut steps, &db);
    push_created_table_steps(&mut steps, &db);
    push_dropped_table_steps(&mut steps, &db);
    push_dropped_index_steps(&mut steps, &db);
    push_created_index_steps(&mut steps, &db);
    push_altered_table_steps(&mut steps, &db);
    push_redefined_table_steps(&mut steps, &db);

    flavour.push_enum_steps(&mut steps, &db);
    flavour.push_alter_sequence_steps(&mut steps, &db);

    sort_migration_steps(&mut steps, &db);

    steps
}

fn push_created_schema_steps(steps: &mut Vec<SqlMigrationStep>, db: &DifferDatabase<'_>) {
    for schema in db.created_namespaces() {
        steps.push(SqlMigrationStep::CreateSchema(schema.id))
    }
}

fn push_created_table_steps(steps: &mut Vec<SqlMigrationStep>, db: &DifferDatabase<'_>) {
    for table in db.created_tables() {
        steps.push(SqlMigrationStep::CreateTable { table_id: table.id });

        if db.flavour.should_push_foreign_keys_from_created_tables() {
            for fk in table.foreign_keys() {
                steps.push(SqlMigrationStep::AddForeignKey { foreign_key_id: fk.id });
            }
        }

        if db.flavour.should_create_indexes_from_created_tables() {
            let create_indexes_from_created_tables = table
                .indexes()
                .filter(|index| !index.is_primary_key() && !db.flavour.should_skip_index_for_new_table(*index))
                .map(|index| SqlMigrationStep::CreateIndex {
                    table_id: (None, index.table().id),
                    index_id: index.id,
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
            table_id: dropped_table.id,
        });

        if !db.flavour.should_drop_foreign_keys_from_dropped_tables() {
            continue;
        }

        for fk in dropped_table.foreign_keys() {
            steps.push(SqlMigrationStep::DropForeignKey { foreign_key_id: fk.id });
        }
    }
}

fn push_altered_table_steps(steps: &mut Vec<SqlMigrationStep>, db: &DifferDatabase<'_>) {
    for table in db.non_redefined_table_pairs() {
        for created_fk in table.created_foreign_keys() {
            steps.push(SqlMigrationStep::AddForeignKey {
                foreign_key_id: created_fk.id,
            })
        }

        for dropped_fk in table.dropped_foreign_keys() {
            steps.push(SqlMigrationStep::DropForeignKey {
                foreign_key_id: dropped_fk.id,
            })
        }

        for fk in table.foreign_key_pairs() {
            push_foreign_key_pair_changes(fk, steps, db)
        }

        push_alter_primary_key(&table, steps);

        // Indexes.
        for i in table
            .index_pairs()
            .filter(|pair| db.flavour.index_should_be_renamed(*pair))
        {
            let index: MigrationPair<IndexId> = i.map(|i| i.id);

            let step = if db.flavour.can_rename_index() {
                SqlMigrationStep::RenameIndex { index }
            } else {
                SqlMigrationStep::RedefineIndex { index }
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
            let ids = column.map(|c| c.id);
            db.flavour
                .push_index_changes_for_column_changes(&table, ids, db.column_changes(ids), steps);
        }

        steps.push(SqlMigrationStep::AlterTable(AlterTable {
            table_ids: table.tables.map(|t| t.id),
            changes,
        }));
    }
}

fn dropped_columns(differ: &TableDiffer<'_, '_>, changes: &mut Vec<TableChange>) {
    for column in differ.dropped_columns() {
        changes.push(TableChange::DropColumn { column_id: column.id })
    }
}

fn added_columns(differ: &TableDiffer<'_, '_>, changes: &mut Vec<TableChange>) {
    for column in differ.added_columns() {
        changes.push(TableChange::AddColumn {
            column_id: column.id,
            has_virtual_default: next_column_has_virtual_default(column.id, differ.db),
        })
    }
}

fn alter_columns(table_differ: &TableDiffer<'_, '_>) -> Vec<TableChange> {
    let mut alter_columns: Vec<_> = table_differ
        .column_pairs()
        .filter_map(move |column_differ| {
            let changes = table_differ.db.column_changes_for_walkers(column_differ);

            if !changes.differs_in_something() {
                return None;
            }

            let column_id = MigrationPair::new(column_differ.previous.id, column_differ.next.id);

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
        .map(|_| TableChange::AddPrimaryKey)
        .or_else(|| Some(TableChange::AddPrimaryKey).filter(|_| differ.primary_key_changed()));

    if differ.db.flavour.should_recreate_the_primary_key_on_column_recreate() {
        from_psl_change.or_else(|| {
            let from_recreate = alter_columns(differ).into_iter().any(|tc| match tc {
                TableChange::DropAndRecreateColumn { column_id, .. } => {
                    let id = column_id.previous;
                    differ.previous().walk(id).is_part_of_primary_key()
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
                TableChange::DropAndRecreateColumn { column_id, .. } => differ
                    .previous()
                    .schema
                    .walk(column_id.previous)
                    .is_part_of_primary_key(),
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
        .map(|pk| pk.primary_key().map(|pk| pk.name()))
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

    if all_match(previous.column_names(), next.column_names()) {
        return;
    }

    steps.push(SqlMigrationStep::AlterPrimaryKey(differ.table_ids()))
}

fn push_created_index_steps(steps: &mut Vec<SqlMigrationStep>, db: &DifferDatabase<'_>) {
    for tables in db.non_redefined_table_pairs() {
        for index in tables.created_indexes() {
            steps.push(SqlMigrationStep::CreateIndex {
                table_id: (Some(tables.previous().id), tables.next().id),
                index_id: index.id,
                from_drop_and_recreate: false,
            })
        }

        if db.flavour.indexes_should_be_recreated_after_column_drop() {
            let dropped_and_recreated_column_ids_next: HashSet<TableColumnId> = tables
                .column_pairs()
                .filter(|columns| {
                    matches!(
                        db.column_changes_for_walkers(*columns).type_change,
                        Some(ColumnTypeChange::NotCastable)
                    )
                })
                .map(|col| col.next.id)
                .collect();

            for index in tables.index_pairs().filter(|index| {
                index
                    .next
                    .columns()
                    .any(|col| dropped_and_recreated_column_ids_next.contains(&col.as_column().id))
            }) {
                steps.push(SqlMigrationStep::CreateIndex {
                    table_id: (Some(tables.previous().id), tables.next().id),
                    index_id: index.next.id,
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
            if db.flavour.should_skip_fk_indexes() && index::index_covers_fk(tables.previous(), index) {
                continue;
            }

            drop_indexes.insert(index.id);
        }
    }

    // On SQLite, we will recreate indexes in the RedefineTables step,
    // because they are needed for implementing new foreign key constraints.
    if !db.tables_to_redefine.is_empty() && db.flavour.should_drop_indexes_from_dropped_tables() {
        for table in db.dropped_tables() {
            for index in table.indexes().filter(|idx| !idx.is_primary_key()) {
                drop_indexes.insert(index.id);
            }
        }
    }

    for index_id in drop_indexes.into_iter() {
        steps.push(SqlMigrationStep::DropIndex { index_id })
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
                        columns.map(|col| col.id),
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
                table_ids: differ.tables.map(|t| t.id),
                dropped_primary_key: dropped_primary_key(&differ).is_some(),
                added_columns: differ.added_columns().map(|col| col.id).collect(),
                added_columns_with_virtual_defaults: differ
                    .added_columns()
                    .filter(|col| next_column_has_virtual_default(col.id, differ.db))
                    .map(|col| col.id)
                    .collect(),
                dropped_columns: differ.dropped_columns().map(|col| col.id).collect(),
                column_pairs,
            }
        })
        .collect();

    steps.push(SqlMigrationStep::RedefineTables(tables_to_redefine))
}

/// Compare two foreign keys and return whether they should be considered
/// equivalent for schema diffing purposes.
fn foreign_keys_match(fks: MigrationPair<&ForeignKeyWalker<'_>>, db: &DifferDatabase<'_>) -> bool {
    let references_same_table = db.flavour.table_names_match(fks.map(|fk| fk.referenced_table().name()));

    let references_same_column_count = fks.previous.referenced_columns().len() == fks.next.referenced_columns().len();
    let constrains_same_column_count = fks.previous.constrained_columns().len() == fks.next.constrained_columns().len();

    let constrains_same_columns = fks.interleave(|fk| fk.constrained_columns()).all(|cols| {
        let type_changed = || db.column_changes_for_walkers(cols).type_changed();

        let arities_ok = db.flavour.can_cope_with_foreign_key_column_becoming_non_nullable()
            || (cols.previous.arity() == cols.next.arity()
                || (cols.previous.arity().is_required() && cols.next.arity().is_nullable()));

        cols.previous.name() == cols.next.name() && !type_changed() && arities_ok
    });

    // Foreign key references different columns or the same columns in a different order.
    let references_same_columns = fks
        .interleave(|fk| fk.referenced_columns().map(|c| c.name()))
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
    fk: MigrationPair<ForeignKeyWalker<'_>>,
    steps: &mut Vec<SqlMigrationStep>,
    db: &DifferDatabase<'_>,
) {
    // Is the referenced table being redefined, meaning we need to drop and recreate
    // the foreign key?
    if db.table_is_redefined(
        fk.previous.referenced_table().namespace().map(Cow::Borrowed),
        fk.previous.referenced_table().name().into(),
    ) && !db.flavour.can_redefine_tables_with_inbound_foreign_keys()
    {
        steps.push(SqlMigrationStep::DropForeignKey {
            foreign_key_id: fk.previous.id,
        });
        steps.push(SqlMigrationStep::AddForeignKey {
            foreign_key_id: fk.next.id,
        });
        return;
    }

    if db.flavour.has_unnamed_foreign_keys() {
        return;
    }

    if fk
        .map(|fk| fk.constraint_name())
        .transpose()
        .map(|names| names.previous != names.next)
        .unwrap_or(false)
    {
        // Rename the foreign key.

        // Since we are using the conventional foreign key names for the foreign keys of
        // many-to-many relation tables, but we used not to (we did not provide a constraint
        // names), and we do not want to cause new migrations on upgrade, we ignore the foreign
        // keys of implicit many-to-many relation tables for renamings.
        if fk.map(is_prisma_implicit_m2m_fk).into_tuple() == (true, true) {
            return;
        }

        if db.flavour.can_rename_foreign_key() {
            steps.push(SqlMigrationStep::RenameForeignKey {
                foreign_key_id: fk.map(|fk| fk.id),
            })
        } else {
            steps.push(SqlMigrationStep::AddForeignKey {
                foreign_key_id: fk.next.id,
            });
            steps.push(SqlMigrationStep::DropForeignKey {
                foreign_key_id: fk.previous.id,
            })
        }
    }
}

fn next_column_has_virtual_default(column_id: TableColumnId, db: &DifferDatabase<'_>) -> bool {
    db.schemas.next.prisma_level_defaults.binary_search(&column_id).is_ok()
}

fn is_prisma_implicit_m2m_fk(fk: ForeignKeyWalker<'_>) -> bool {
    let table = fk.table();

    if table.columns().count() != 2 {
        return false;
    }

    table.column("A").is_some() && table.column("B").is_some()
}

fn all_match<T: PartialEq>(a: impl ExactSizeIterator<Item = T>, b: impl ExactSizeIterator<Item = T>) -> bool {
    a.len() == b.len() && a.zip(b).all(|(a, b)| a == b)
}

fn sort_migration_steps(steps: &mut Vec<SqlMigrationStep>, db: &DifferDatabase<'_>) {
    // We can't merge these two steps into `sort_by` because sorting algorithms require total order
    // relation, but the dependency between dropped unique index and creating a corresponding
    // primary key is a preorder. Moreover, the binary relation defined as the composition of
    // custom logic for `(SqlMigrationStep::DropIndex, SqlMigrationStep::AlterTable)` pairs and
    // `a.cmp(b)` for everything else doesn't appear to be even transitive (due to the existing
    // automatically derived total order relation between `SqlMigrationStep`s). If we need more
    // complex ordering logic in the future, we should consider defining a partial order on
    // `SqlMigrationStep` where only the pairs for which order actually matters are ordered,
    // building a graph from the steps and topologically sorting it.
    steps.sort();
    apply_partial_order_permutations(steps, db);
}

fn apply_partial_order_permutations(steps: &mut Vec<SqlMigrationStep>, db: &DifferDatabase<'_>) {
    fn find_dropped_unique_index<'a>(
        steps: &[SqlMigrationStep],
        seen_elements: &mut usize,
        db: &DifferDatabase<'a>,
    ) -> Option<Walker<'a, IndexId>> {
        for step in steps {
            *seen_elements += 1;

            if let SqlMigrationStep::DropIndex { index_id } = step {
                let index = db.schemas.previous.describer_schema.walk(*index_id);

                // We're interested in dropped unique indexes in tables that didn't have a primary
                // key defined.
                if index.is_unique() && index.table().primary_key().is_none() {
                    return Some(index);
                }
            }
        }

        None
    }

    fn find_matching_created_pk_step<'a>(
        steps: &[SqlMigrationStep],
        index: Walker<'a, IndexId>,
        db: &DifferDatabase<'a>,
    ) -> Option<usize> {
        steps
            .iter()
            .enumerate()
            .filter_map(|(i, step)| match step {
                SqlMigrationStep::AlterTable(alter_table) => Some((i, alter_table)),
                _ => None,
            })
            .filter(|(_, alter_table)| alter_table.table_ids.previous == index.table().id)
            // We're only interested in `AlterTable` steps that create a primary key.
            .filter(|(_, alter_table)| {
                alter_table
                    .changes
                    .iter()
                    .any(|change| matches!(change, TableChange::AddPrimaryKey))
            })
            // This `AlterTable` step must not have dropped or recreated any columns from the
            // unique index we were looking at.
            .filter(|(_, alter_table)| {
                alter_table.changes.iter().all(|change| match change {
                    TableChange::DropColumn { column_id } => !index.contains_column(*column_id),
                    TableChange::DropAndRecreateColumn { column_id, .. } => !index.contains_column(column_id.previous),
                    _ => true,
                })
            })
            // The primary key must be created on the same columns as the unique index.
            .find(|(_, alter_table)| {
                let table = db.schemas.next.describer_schema.walk(alter_table.table_ids.next);
                table.primary_key().is_some()
                    && all_match(
                        index.column_names(),
                        table.primary_key_columns().unwrap().map(|col| col.name()),
                    )
            })
            .map(|(i, _)| i)
    }

    let mut i = 0;

    while let Some(index) = find_dropped_unique_index(&steps[i..], &mut i, db) {
        let index_pos = i - 1;

        if let Some(alter_table_offset) = find_matching_created_pk_step(&steps[i..], index, db) {
            let alter_table_pos = i + alter_table_offset;
            let drop_index_step = steps.remove(index_pos);
            steps.insert(alter_table_pos, drop_index_step);

            // We need to adjust the index so we don't skip the element following the `DropIndex`
            // step we just moved, as the following elements were shifted left by one.
            i -= 1;
        }
    }
}
