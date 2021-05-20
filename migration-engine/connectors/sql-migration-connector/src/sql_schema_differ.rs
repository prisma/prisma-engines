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
    sql_migration::{
        self, AddColumn, AlterColumn, AlterEnum, AlterTable, CreateIndex, DropColumn, DropForeignKey, DropIndex,
        RedefineTable, SqlMigrationStep, TableChange,
    },
    SqlFlavour, SqlSchema,
};
use column::ColumnTypeChange;
use enums::EnumDiffer;
use sql_schema_describer::{
    walkers::{EnumWalker, ForeignKeyWalker, SqlSchemaExt, TableWalker},
    ColumnTypeFamily,
};
use std::collections::HashSet;
use table::TableDiffer;

pub(crate) fn calculate_steps(schemas: Pair<&SqlSchema>, flavour: &dyn SqlFlavour) -> Vec<SqlMigrationStep> {
    let db = DifferDatabase::new(schemas, flavour);
    let differ = SqlSchemaDiffer { schemas, flavour, db };
    let mut steps: Vec<SqlMigrationStep> = Vec::new();
    differ.push_create_tables(&mut steps);

    let tables_to_redefine = differ.flavour.tables_to_redefine(&differ);
    let mut alter_indexes = differ.alter_indexes(&tables_to_redefine);

    let redefine_indexes = if differ.flavour.can_alter_index() {
        Vec::new()
    } else {
        std::mem::replace(&mut alter_indexes, Vec::new())
    };

    let (drop_tables, mut drop_foreign_keys) = differ.drop_tables();
    differ.drop_foreign_keys(&mut drop_foreign_keys, &tables_to_redefine);

    let mut drop_indexes = differ.drop_indexes(&tables_to_redefine);
    let mut create_indexes = differ.create_indexes(&tables_to_redefine);

    let mut alter_tables = differ.alter_tables(&tables_to_redefine).collect::<Vec<_>>();
    alter_tables.sort_by_key(|at| at.table_index);

    flavour.push_index_changes_for_column_changes(&alter_tables, &mut drop_indexes, &mut create_indexes, &differ);

    let redefine_tables = differ.redefine_tables(&tables_to_redefine);
    let mut alter_enums = flavour.alter_enums(&differ);
    push_previous_usages_as_defaults_in_altered_enums(&differ, &mut alter_enums);

    let redefine_tables = Some(redefine_tables)
        .filter(|tables| !tables.is_empty())
        .map(SqlMigrationStep::RedefineTables);

    differ.push_add_foreign_keys(&tables_to_redefine, &mut steps);
    flavour.create_enums(&differ, &mut steps);
    flavour.drop_enums(&differ, &mut steps);

    steps.extend(
        alter_enums
            .into_iter()
            .map(SqlMigrationStep::AlterEnum)
            .chain(drop_foreign_keys.into_iter().map(SqlMigrationStep::DropForeignKey))
            .chain(drop_indexes.into_iter().map(SqlMigrationStep::DropIndex))
            .chain(alter_tables.into_iter().map(SqlMigrationStep::AlterTable))
            .chain(
                drop_tables
                    .into_iter()
                    .map(|table_index| SqlMigrationStep::DropTable { table_index }),
            )
            .chain(redefine_tables)
            .chain(create_indexes.into_iter().map(SqlMigrationStep::CreateIndex))
            .chain(alter_indexes.into_iter().map(|idxs| SqlMigrationStep::AlterIndex {
                table: idxs.as_ref().map(|(table, _)| *table),
                index: idxs.as_ref().map(|(_, idx)| *idx),
            }))
            .chain(
                redefine_indexes
                    .into_iter()
                    .map(|idxs| SqlMigrationStep::RedefineIndex {
                        table: idxs.as_ref().map(|(table, _)| *table),
                        index: idxs.as_ref().map(|(_, idx)| *idx),
                    }),
            ),
    );

    steps.sort();

    steps
}

pub(crate) struct SqlSchemaDiffer<'a> {
    schemas: Pair<&'a SqlSchema>,
    flavour: &'a dyn SqlFlavour,
    db: DifferDatabase<'a>,
}

impl<'schema> SqlSchemaDiffer<'schema> {
    fn push_create_tables(&self, steps: &mut Vec<SqlMigrationStep>) {
        for table in self.created_tables() {
            steps.push(SqlMigrationStep::CreateTable {
                table_index: table.table_index(),
            });

            if self.flavour.should_push_foreign_keys_from_created_tables() {
                for fk in table.foreign_keys() {
                    steps.push(SqlMigrationStep::AddForeignKey {
                        table_index: table.table_index(),
                        foreign_key_index: fk.foreign_key_index(),
                    });
                }
            }
        }
    }

    // We drop the foreign keys of dropped tables first, so we can drop tables in whatever order we
    // please later.
    fn drop_tables(&self) -> (Vec<usize>, Vec<DropForeignKey>) {
        let (dropped_tables_count, dropped_fks_count) = self.dropped_tables().fold((0, 0), |(tables, fks), item| {
            (tables + 1, fks + item.foreign_key_count())
        });

        let mut dropped_tables = Vec::with_capacity(dropped_tables_count);
        let mut dropped_foreign_keys = Vec::with_capacity(dropped_fks_count);

        for dropped_table in self.dropped_tables() {
            dropped_tables.push(dropped_table.table_index());

            for (fk, fk_name) in dropped_table
                .foreign_keys()
                .filter_map(|fk| fk.constraint_name().map(|name| (fk, name)))
            {
                let drop_foreign_key = DropForeignKey {
                    table_index: dropped_table.table_index(),
                    foreign_key_index: fk.foreign_key_index(),
                    table: dropped_table.name().to_owned(),
                    constraint_name: fk_name.to_owned(),
                };

                dropped_foreign_keys.push(drop_foreign_key);
            }
        }

        (dropped_tables, dropped_foreign_keys)
    }

    fn push_add_foreign_keys(&self, tables_to_redefine: &HashSet<String>, steps: &mut Vec<SqlMigrationStep>) {
        for table in self
            .table_pairs()
            .filter(|tables| !tables_to_redefine.contains(tables.next().name()))
        {
            for created_fk in table.created_foreign_keys() {
                steps.push(SqlMigrationStep::AddForeignKey {
                    table_index: created_fk.table().table_index(),
                    foreign_key_index: created_fk.foreign_key_index(),
                })
            }
        }
    }

    fn alter_tables<'a, 'b: 'a>(
        &'a self,
        tables_to_redefine: &'b HashSet<String>,
    ) -> impl Iterator<Item = AlterTable> + 'a {
        self.table_pairs()
            .filter(move |tables| !tables_to_redefine.contains(tables.next().name()))
            .filter_map(|differ| {
                // Order matters.
                let changes: Vec<TableChange> = SqlSchemaDiffer::drop_primary_key(&differ)
                    .into_iter()
                    .chain(SqlSchemaDiffer::drop_columns(&differ))
                    .chain(SqlSchemaDiffer::add_columns(&differ))
                    .chain(SqlSchemaDiffer::alter_columns(&differ).into_iter())
                    .chain(SqlSchemaDiffer::add_primary_key(&differ))
                    .collect();

                Some(changes)
                    .filter(|changes| !changes.is_empty())
                    .map(|changes| AlterTable {
                        table_index: differ.tables.map(|t| t.table_index()),
                        changes,
                    })
            })
    }

    fn drop_columns<'a>(differ: &'a TableDiffer<'schema, 'a>) -> impl Iterator<Item = TableChange> + 'a {
        differ.dropped_columns().map(|column| {
            let change = DropColumn {
                index: column.column_index(),
            };

            TableChange::DropColumn(change)
        })
    }

    fn add_columns<'a>(differ: &'a TableDiffer<'schema, 'a>) -> impl Iterator<Item = TableChange> + 'a {
        differ.added_columns().map(move |column| {
            let change = AddColumn {
                column_index: column.column_index(),
            };

            TableChange::AddColumn(change)
        })
    }

    fn alter_columns(table_differ: &TableDiffer<'_, '_>) -> Vec<TableChange> {
        let mut alter_columns: Vec<_> = table_differ
            .column_pairs()
            .filter_map(move |column_differ| {
                let (changes, type_change) = column_differ.all_changes();

                if !changes.differs_in_something() {
                    return None;
                }

                let column_index = Pair::new(column_differ.previous.column_index(), column_differ.next.column_index());

                match type_change {
                    Some(ColumnTypeChange::NotCastable) => {
                        Some(TableChange::DropAndRecreateColumn { column_index, changes })
                    }
                    Some(ColumnTypeChange::RiskyCast) => Some(TableChange::AlterColumn(AlterColumn {
                        column_index,
                        changes,
                        type_change: Some(crate::sql_migration::ColumnTypeChange::RiskyCast),
                    })),
                    Some(ColumnTypeChange::SafeCast) => Some(TableChange::AlterColumn(AlterColumn {
                        column_index,
                        changes,
                        type_change: Some(crate::sql_migration::ColumnTypeChange::SafeCast),
                    })),
                    None => Some(TableChange::AlterColumn(AlterColumn {
                        column_index,
                        changes,
                        type_change: None,
                    })),
                }
            })
            .collect();

        alter_columns.sort_by_key(|alter_col| match alter_col {
            TableChange::AlterColumn(alter_col) => alter_col.column_index,
            TableChange::DropAndRecreateColumn { column_index, .. } => *column_index,
            _ => unreachable!(),
        });

        alter_columns
    }

    fn drop_foreign_keys(&self, drop_foreign_keys: &mut Vec<DropForeignKey>, tables_to_redefine: &HashSet<String>) {
        for differ in self
            .table_pairs()
            .filter(|tables| !tables_to_redefine.contains(tables.next().name()))
        {
            for (dropped_fk, dropped_foreign_key_name) in differ
                .dropped_foreign_keys()
                .filter_map(|foreign_key| foreign_key.constraint_name().map(|name| (foreign_key, name)))
            {
                drop_foreign_keys.push(DropForeignKey {
                    table_index: differ.previous().table_index(),
                    table: differ.previous().name().to_owned(),
                    foreign_key_index: dropped_fk.foreign_key_index(),
                    constraint_name: dropped_foreign_key_name.to_owned(),
                })
            }
        }
    }

    fn add_primary_key(differ: &TableDiffer<'_, '_>) -> Option<TableChange> {
        let from_psl_change = differ
            .created_primary_key()
            .filter(|pk| !pk.columns.is_empty())
            .map(|pk| TableChange::AddPrimaryKey {
                columns: pk.columns.clone(),
            });

        if differ.flavour.should_recreate_the_primary_key_on_column_recreate() {
            from_psl_change.or_else(|| {
                let from_recreate = Self::alter_columns(differ).into_iter().any(|tc| match tc {
                    TableChange::DropAndRecreateColumn { column_index, .. } => {
                        let idx = *column_index.previous();
                        differ.previous().column_at(idx).is_part_of_primary_key()
                    }
                    _ => false,
                });

                if from_recreate {
                    Some(TableChange::AddPrimaryKey {
                        columns: differ.previous().table().primary_key_columns(),
                    })
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

        if differ.flavour.should_recreate_the_primary_key_on_column_recreate() {
            from_psl_change.or_else(|| {
                let from_recreate = Self::alter_columns(differ).into_iter().any(|tc| match tc {
                    TableChange::DropAndRecreateColumn { column_index, .. } => {
                        let idx = *column_index.previous();
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

    fn create_indexes(&self, tables_to_redefine: &HashSet<String>) -> Vec<CreateIndex> {
        let mut steps = Vec::new();

        if self.flavour.should_create_indexes_from_created_tables() {
            let create_indexes_from_created_tables = self
                .created_tables()
                .flat_map(|table| table.indexes())
                .filter(|index| !self.flavour.should_skip_index_for_new_table(index))
                .map(|index| CreateIndex {
                    table_index: index.table().table_index(),
                    index_index: index.index(),
                    caused_by_create_table: true,
                });

            steps.extend(create_indexes_from_created_tables);
        }

        for tables in self
            .table_pairs()
            .filter(|tables| !tables_to_redefine.contains(tables.next().name()))
        {
            for index in tables.created_indexes() {
                steps.push(CreateIndex {
                    table_index: index.table().table_index(),
                    index_index: index.index(),
                    caused_by_create_table: false,
                })
            }

            if self.flavour.indexes_should_be_recreated_after_column_drop() {
                let dropped_and_recreated_column_indexes_next: HashSet<usize> = tables
                    .column_pairs()
                    .filter(|columns| matches!(columns.all_changes().1, Some(ColumnTypeChange::NotCastable)))
                    .map(|col| col.as_pair().next().column_index())
                    .collect();

                for index in tables.index_pairs().filter(|index| {
                    index
                        .next()
                        .columns()
                        .any(|col| dropped_and_recreated_column_indexes_next.contains(&col.column_index()))
                }) {
                    steps.push(CreateIndex {
                        table_index: tables.next().table_index(),
                        index_index: index.next().index(),
                        caused_by_create_table: false,
                    })
                }
            }
        }

        steps
    }

    fn drop_indexes(&self, tables_to_redefine: &HashSet<String>) -> Vec<DropIndex> {
        let mut drop_indexes = HashSet::new();

        for tables in self.table_pairs() {
            for index in tables.dropped_indexes() {
                // On MySQL, foreign keys automatically create indexes. These foreign-key-created
                // indexes should only be dropped as part of the foreign key.
                if self.flavour.should_skip_fk_indexes() && index::index_covers_fk(&tables.previous(), &index) {
                    continue;
                }

                drop_indexes.insert(DropIndex {
                    table_index: index.table().table_index(),
                    index_index: index.index(),
                });
            }
        }

        // On SQLite, we will recreate indexes in the RedefineTables step,
        // because they are needed for implementing new foreign key constraints.
        if !tables_to_redefine.is_empty() && self.flavour.should_drop_indexes_from_dropped_tables() {
            for table in self.dropped_tables() {
                for index in table.indexes() {
                    drop_indexes.insert(DropIndex {
                        table_index: index.table().table_index(),
                        index_index: index.index(),
                    });
                }
            }
        }

        drop_indexes.into_iter().collect()
    }

    fn redefine_tables(&self, tables_to_redefine: &HashSet<String>) -> Vec<RedefineTable> {
        self.table_pairs()
            .filter(|tables| tables_to_redefine.contains(tables.next().name()))
            .map(|differ| {
                let column_pairs = differ
                    .column_pairs()
                    .map(|columns| {
                        let (changes, type_change) = columns.all_changes();
                        (
                            Pair::new(columns.previous.column_index(), columns.next.column_index()),
                            changes,
                            type_change.map(|tc| match tc {
                                ColumnTypeChange::SafeCast => sql_migration::ColumnTypeChange::SafeCast,
                                ColumnTypeChange::RiskyCast => sql_migration::ColumnTypeChange::RiskyCast,
                                ColumnTypeChange::NotCastable => sql_migration::ColumnTypeChange::NotCastable,
                            }),
                        )
                    })
                    .collect();

                RedefineTable {
                    table_index: differ.tables.as_ref().map(|t| t.table_index()),
                    dropped_primary_key: SqlSchemaDiffer::drop_primary_key(&differ).is_some(),
                    added_columns: differ.added_columns().map(|col| col.column_index()).collect(),
                    dropped_columns: differ.dropped_columns().map(|col| col.column_index()).collect(),
                    column_pairs,
                }
            })
            .collect()
    }

    /// An iterator over the tables that are present in both schemas.
    fn table_pairs(&self) -> impl Iterator<Item = TableDiffer<'schema, '_>> + '_ {
        self.db.table_pairs().map(move |tables| TableDiffer {
            flavour: self.flavour,
            tables: self.schemas.tables(&tables),
            db: &self.db,
        })
    }

    fn alter_indexes(&self, tables_to_redefine: &HashSet<String>) -> Vec<Pair<(usize, usize)>> {
        let mut steps = Vec::new();

        for differ in self
            .table_pairs()
            .filter(|tables| !tables_to_redefine.contains(tables.next().name()))
        {
            for pair in differ
                .index_pairs()
                .filter(|pair| self.flavour.index_should_be_renamed(&pair))
            {
                steps.push(pair.as_ref().map(|i| (i.table().table_index(), i.index())));
            }
        }

        steps
    }

    fn created_tables(&self) -> impl Iterator<Item = TableWalker<'schema>> + '_ {
        self.db
            .created_tables()
            .map(move |table_index| self.schemas.next().table_walker_at(table_index))
    }

    fn dropped_tables(&self) -> impl Iterator<Item = TableWalker<'schema>> + '_ {
        self.db
            .dropped_tables()
            .map(move |table_index| self.schemas.previous().table_walker_at(table_index))
    }

    fn enum_pairs(&self) -> impl Iterator<Item = EnumDiffer<'_>> {
        self.previous_enums().filter_map(move |previous| {
            self.next_enums()
                .find(|next| enums_match(&previous, &next))
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

fn push_previous_usages_as_defaults_in_altered_enums(differ: &SqlSchemaDiffer<'_>, alter_enums: &mut [AlterEnum]) {
    for alter_enum in alter_enums {
        let mut previous_usages_as_default = Vec::new();

        let enum_names = differ.schemas.enums(&alter_enum.index).map(|enm| enm.name());

        for table in differ.dropped_tables() {
            for column in table
                .columns()
                .filter(|col| col.column_type_is_enum(enum_names.previous()) && col.default().is_some())
            {
                previous_usages_as_default.push(((column.table().table_index(), column.column_index()), None));
            }
        }

        for tables in differ.table_pairs() {
            for column in tables
                .dropped_columns()
                .filter(|col| col.column_type_is_enum(enum_names.previous()) && col.default().is_some())
            {
                previous_usages_as_default.push(((column.table().table_index(), column.column_index()), None));
            }

            for columns in tables.column_pairs().filter(|col| {
                col.previous.column_type_is_enum(enum_names.previous()) && col.previous.default().is_some()
            }) {
                let next_usage_as_default = Some(&columns.next)
                    .filter(|col| col.column_type_is_enum(enum_names.next()) && col.default().is_some())
                    .map(|col| (col.table().table_index(), col.column_index()));

                previous_usages_as_default.push((
                    (columns.previous.table().table_index(), columns.previous.column_index()),
                    next_usage_as_default,
                ));
            }
        }

        alter_enum.previous_usages_as_default = previous_usages_as_default;
    }
}

/// Compare two [ForeignKey](/sql-schema-describer/struct.ForeignKey.html)s and return whether they
/// should be considered equivalent for schema diffing purposes.
fn foreign_keys_match(fks: Pair<&ForeignKeyWalker<'_>>, flavour: &dyn SqlFlavour) -> bool {
    let references_same_table = flavour.table_names_match(fks.map(|fk| fk.referenced_table().name()));
    let references_same_column_count =
        fks.previous().referenced_columns_count() == fks.next().referenced_columns_count();
    let constrains_same_column_count =
        fks.previous().constrained_columns().count() == fks.next().constrained_columns().count();
    let constrains_same_columns = fks.interleave(|fk| fk.constrained_columns()).all(|cols| {
        let families_match = match cols.map(|col| col.column_type_family()).as_tuple() {
            (ColumnTypeFamily::Uuid, ColumnTypeFamily::String) => true,
            (ColumnTypeFamily::String, ColumnTypeFamily::Uuid) => true,
            (x, y) => x == y,
        };

        let arities_ok = flavour.can_cope_with_foreign_key_column_becoming_nonnullable()
            || (cols.previous().arity() == cols.next().arity()
                || (cols.previous().arity().is_required() && cols.next().arity().is_nullable()));

        cols.previous().name() == cols.next().name() && families_match && arities_ok
    });

    // Foreign key references different columns or the same columns in a different order.
    let references_same_columns = fks
        .interleave(|fk| fk.referenced_column_names())
        .all(|pair| pair.previous() == pair.next());

    let same_on_delete_action = fks.previous().on_delete_action() == fks.next().on_delete_action();
    let same_on_update_action = fks.previous().on_update_action() == fks.next().on_update_action();

    references_same_table
        && references_same_column_count
        && constrains_same_column_count
        && constrains_same_columns
        && references_same_columns
        && same_on_delete_action
        && same_on_update_action
}

fn enums_match(previous: &EnumWalker<'_>, next: &EnumWalker<'_>) -> bool {
    previous.name() == next.name()
}
