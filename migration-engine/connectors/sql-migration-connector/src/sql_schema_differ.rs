mod column;
mod enums;
mod index;
mod sql_schema_differ_flavour;
mod table;

pub(crate) use column::{ColumnChange, ColumnChanges};
pub(crate) use sql_schema_differ_flavour::SqlSchemaDifferFlavour;

use crate::{
    pair::Pair,
    sql_migration::{
        self, AddColumn, AddForeignKey, AlterColumn, AlterEnum, AlterTable, CreateEnum, CreateIndex, CreateTable,
        DropColumn, DropEnum, DropForeignKey, DropIndex, DropTable, RedefineTable, SqlMigrationStep, TableChange,
    },
    SqlFlavour, SqlSchema,
};
use column::ColumnTypeChange;
use enums::EnumDiffer;
use sql_schema_describer::{
    walkers::{EnumWalker, ForeignKeyWalker, TableWalker},
    ColumnTypeFamily,
};
use std::collections::HashSet;
use table::TableDiffer;

pub(crate) fn calculate_steps(schemas: Pair<&SqlSchema>, flavour: &dyn SqlFlavour) -> Vec<SqlMigrationStep> {
    let differ = SqlSchemaDiffer { schemas, flavour };

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

    let alter_tables = differ.alter_tables(&tables_to_redefine).collect::<Vec<_>>();

    flavour.push_index_changes_for_column_changes(&alter_tables, &mut drop_indexes, &mut create_indexes, &differ);

    let redefine_tables = differ.redefine_tables(&tables_to_redefine);
    let add_foreign_keys = differ.add_foreign_keys(&tables_to_redefine);
    let create_enums = differ.create_enums();

    let redefine_tables = Some(redefine_tables)
        .filter(|tables| !tables.is_empty())
        .map(SqlMigrationStep::RedefineTables);

    create_enums
        .into_iter()
        .map(SqlMigrationStep::CreateEnum)
        .chain(differ.alter_enums().into_iter().map(SqlMigrationStep::AlterEnum))
        .chain(drop_foreign_keys.into_iter().map(SqlMigrationStep::DropForeignKey))
        .chain(drop_indexes.into_iter().map(SqlMigrationStep::DropIndex))
        .chain(alter_tables.into_iter().map(SqlMigrationStep::AlterTable))
        // Order matters: we must drop enums before we create tables,
        // because the new tables might be named the same as the dropped
        // enum, and that conflicts on postgres.
        .chain(differ.drop_enums().map(SqlMigrationStep::DropEnum))
        .chain(differ.create_tables().map(SqlMigrationStep::CreateTable))
        .chain(redefine_tables)
        // Order matters: we must drop tables before we create indexes,
        // because on Postgres and SQLite, we may create indexes whose names
        // clash with the names of indexes on the dropped tables.
        .chain(drop_tables.into_iter().map(SqlMigrationStep::DropTable))
        // Order matters: we must create indexes after ALTER TABLEs because the indexes can be
        // on fields that are dropped/created there.
        .chain(create_indexes.into_iter().map(SqlMigrationStep::CreateIndex))
        // Order matters: this needs to come after create_indexes, because the foreign keys can depend on unique
        // indexes created there.
        .chain(add_foreign_keys.into_iter().map(SqlMigrationStep::AddForeignKey))
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
        )
        .collect()
}

pub(crate) struct SqlSchemaDiffer<'a> {
    schemas: Pair<&'a SqlSchema>,
    flavour: &'a dyn SqlFlavour,
}

impl<'schema> SqlSchemaDiffer<'schema> {
    #[allow(clippy::needless_lifetimes)] // clippy is wrong here
    fn create_tables<'a>(&'a self) -> impl Iterator<Item = CreateTable> + 'a {
        self.created_tables().map(|created_table| CreateTable {
            table_index: created_table.table_index(),
        })
    }

    // We drop the foreign keys of dropped tables first, so we can drop tables in whatever order we
    // please later.
    fn drop_tables(&self) -> (Vec<DropTable>, Vec<DropForeignKey>) {
        let (dropped_tables_count, dropped_fks_count) = self.dropped_tables().fold((0, 0), |(tables, fks), item| {
            (tables + 1, fks + item.foreign_key_count())
        });

        let mut dropped_tables = Vec::with_capacity(dropped_tables_count);
        let mut dropped_foreign_keys = Vec::with_capacity(dropped_fks_count);

        for dropped_table in self.dropped_tables() {
            dropped_tables.push(DropTable {
                table_index: dropped_table.table_index(),
            });

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

    fn add_foreign_keys(&self, tables_to_redefine: &HashSet<String>) -> Vec<AddForeignKey> {
        let mut add_foreign_keys = Vec::new();
        let table_pairs = self
            .table_pairs()
            .filter(|tables| !tables_to_redefine.contains(tables.next().name()));

        if self.flavour.should_push_foreign_keys_from_created_tables() {
            push_foreign_keys_from_created_tables(&mut add_foreign_keys, self.created_tables());
        }

        push_created_foreign_keys(&mut add_foreign_keys, table_pairs);

        add_foreign_keys
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
                    .chain(SqlSchemaDiffer::alter_columns(&differ))
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

    fn drop_columns<'a>(differ: &'a TableDiffer<'schema>) -> impl Iterator<Item = TableChange> + 'a {
        differ.dropped_columns().map(|column| {
            let change = DropColumn {
                index: column.column_index(),
            };

            TableChange::DropColumn(change)
        })
    }

    fn add_columns<'a>(differ: &'a TableDiffer<'schema>) -> impl Iterator<Item = TableChange> + 'a {
        differ.added_columns().map(move |column| {
            let change = AddColumn {
                column_index: column.column_index(),
            };

            TableChange::AddColumn(change)
        })
    }

    fn alter_columns<'a>(table_differ: &'a TableDiffer<'schema>) -> impl Iterator<Item = TableChange> + 'a {
        table_differ.column_pairs().filter_map(move |column_differ| {
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
    }

    fn drop_foreign_keys<'a>(
        &'a self,
        drop_foreign_keys: &mut Vec<DropForeignKey>,
        tables_to_redefine: &HashSet<String>,
    ) {
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

    fn add_primary_key(differ: &TableDiffer<'_>) -> Option<TableChange> {
        let from_psl_change = differ
            .created_primary_key()
            .filter(|pk| !pk.columns.is_empty())
            .map(|pk| TableChange::AddPrimaryKey {
                columns: pk.columns.clone(),
            });

        if differ.flavour.should_recreate_the_primary_key_on_column_recreate() {
            from_psl_change.or_else(|| {
                let from_recreate = Self::alter_columns(differ).any(|tc| match tc {
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

    fn drop_primary_key(differ: &TableDiffer<'_>) -> Option<TableChange> {
        let from_psl_change = differ.dropped_primary_key().map(|_pk| TableChange::DropPrimaryKey);

        if differ.flavour.should_recreate_the_primary_key_on_column_recreate() {
            from_psl_change.or_else(|| {
                let from_recreate = Self::alter_columns(differ).any(|tc| match tc {
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

    #[allow(clippy::needless_lifetimes)] // clippy is wrong here
    fn create_enums<'a>(&'a self) -> impl Iterator<Item = CreateEnum> + 'a {
        self.created_enums().map(|r#enum| CreateEnum {
            enum_index: r#enum.enum_index(),
        })
    }

    #[allow(clippy::needless_lifetimes)] // clippy is wrong here
    fn drop_enums<'a>(&'a self) -> impl Iterator<Item = DropEnum> + 'a {
        self.dropped_enums().map(|r#enum| DropEnum {
            enum_index: r#enum.enum_index(),
        })
    }

    fn alter_enums(&self) -> Vec<AlterEnum> {
        self.flavour.alter_enums(self)
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
    fn table_pairs<'a>(&'a self) -> impl Iterator<Item = TableDiffer<'schema>> + 'a
    where
        'schema: 'a,
    {
        self.schemas
            .previous()
            .table_walkers()
            .filter_map(move |previous_table| {
                self.schemas
                    .next()
                    .table_walkers()
                    .find(move |next_table| {
                        self.flavour
                            .table_names_match(Pair::new(previous_table.name(), next_table.name()))
                    })
                    .map(move |next_table| TableDiffer {
                        flavour: self.flavour,
                        tables: Pair::new(previous_table, next_table),
                    })
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

    fn created_tables(&self) -> impl Iterator<Item = TableWalker<'_>> {
        self.next_tables().filter(move |next_table| {
            !self.previous_tables().any(|previous_table| {
                self.flavour
                    .table_names_match(Pair::new(previous_table.name(), next_table.name()))
            })
        })
    }

    fn dropped_tables<'a>(&'a self) -> impl Iterator<Item = TableWalker<'schema>> + 'a {
        self.previous_tables().filter(move |previous_table| {
            !self.next_tables().any(|next_table| {
                self.flavour
                    .table_names_match(Pair::new(previous_table.name(), next_table.name()))
            })
        })
    }

    fn previous_tables<'a>(&'a self) -> impl Iterator<Item = TableWalker<'schema>> + 'a {
        self.schemas
            .previous()
            .table_walkers()
            .filter(move |table| !self.table_is_ignored(&table.name()))
    }

    fn next_tables<'a>(&'a self) -> impl Iterator<Item = TableWalker<'schema>> + 'a {
        self.schemas
            .next()
            .table_walkers()
            .filter(move |table| !self.table_is_ignored(&table.name()))
    }

    fn table_is_ignored(&self, table_name: &str) -> bool {
        table_name == "_prisma_migrations" || self.flavour.table_should_be_ignored(&table_name)
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

fn push_created_foreign_keys<'a, 'schema>(
    added_foreign_keys: &mut Vec<AddForeignKey>,
    table_pairs: impl Iterator<Item = TableDiffer<'schema>>,
) {
    table_pairs.for_each(|differ| {
        added_foreign_keys.extend(differ.created_foreign_keys().map(|created_fk| AddForeignKey {
            table_index: differ.next().table_index(),
            foreign_key_index: created_fk.foreign_key_index(),
        }))
    })
}

fn push_foreign_keys_from_created_tables<'a>(
    steps: &mut Vec<AddForeignKey>,
    created_tables: impl Iterator<Item = TableWalker<'a>>,
) {
    for table in created_tables {
        steps.extend(table.foreign_keys().map(|fk| AddForeignKey {
            table_index: table.table_index(),
            foreign_key_index: fk.foreign_key_index(),
        }));
    }
}

/// Compare two [ForeignKey](/sql-schema-describer/struct.ForeignKey.html)s and return whether they
/// should be considered equivalent for schema diffing purposes.
fn foreign_keys_match(previous: &ForeignKeyWalker<'_>, next: &ForeignKeyWalker<'_>) -> bool {
    let references_same_table = previous.referenced_table().name() == next.referenced_table().name();
    let references_same_column_count = previous.referenced_columns_count() == next.referenced_columns_count();
    let constrains_same_column_count = previous.constrained_columns().count() == next.constrained_columns().count();
    let constrains_same_columns =
        previous
            .constrained_columns()
            .zip(next.constrained_columns())
            .all(|(previous, next)| {
                let families_match = match (previous.column_type_family(), next.column_type_family()) {
                    (ColumnTypeFamily::Uuid, ColumnTypeFamily::String) => true,
                    (ColumnTypeFamily::String, ColumnTypeFamily::Uuid) => true,
                    (x, y) => x == y,
                };

                previous.name() == next.name() && families_match
            });

    // Foreign key references different columns or the same columns in a different order.
    let references_same_columns = previous
        .referenced_column_names()
        .iter()
        .zip(next.referenced_column_names())
        .all(|(previous, next)| previous == next);

    references_same_table
        && references_same_column_count
        && constrains_same_column_count
        && constrains_same_columns
        && references_same_columns
}

fn enums_match(previous: &EnumWalker<'_>, next: &EnumWalker<'_>) -> bool {
    previous.name() == next.name()
}
