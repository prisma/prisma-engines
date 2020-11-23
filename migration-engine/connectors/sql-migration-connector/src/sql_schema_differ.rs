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
    wrap_as_step, SqlFlavour, SqlSchema, MIGRATION_TABLE_NAME,
};
use column::ColumnTypeChange;
use enums::EnumDiffer;
use sql_schema_describer::walkers::{EnumWalker, ForeignKeyWalker, TableWalker};
use std::collections::HashSet;
use table::TableDiffer;

pub(crate) fn calculate_steps(schemas: Pair<&SqlSchema>, flavour: &dyn SqlFlavour) -> Vec<SqlMigrationStep> {
    let differ = SqlSchemaDiffer { schemas, flavour };

    differ.diff_internal().into_steps()
}

#[derive(Debug)]
pub(crate) struct SqlSchemaDiffer<'a> {
    schemas: Pair<&'a SqlSchema>,
    flavour: &'a dyn SqlFlavour,
}

#[derive(Debug)]
struct SqlSchemaDiff {
    add_foreign_keys: Vec<AddForeignKey>,
    drop_foreign_keys: Vec<DropForeignKey>,
    drop_tables: Vec<DropTable>,
    create_tables: Vec<CreateTable>,
    alter_tables: Vec<AlterTable>,
    create_indexes: Vec<CreateIndex>,
    drop_indexes: Vec<DropIndex>,
    alter_indexes: Vec<Pair<(usize, usize)>>,
    redefine_indexes: Vec<Pair<(usize, usize)>>,
    create_enums: Vec<CreateEnum>,
    drop_enums: Vec<DropEnum>,
    alter_enums: Vec<AlterEnum>,
    /// The names of the tables to redefine.
    tables_to_redefine: HashSet<String>,
    redefine_tables: Vec<RedefineTable>,
}

impl SqlSchemaDiff {
    /// Translate the diff into steps that should be executed in order. The general idea in the
    /// ordering of steps is to drop obsolete constraints first, alter/create tables, then add the new constraints.
    fn into_steps(self) -> Vec<SqlMigrationStep> {
        let redefine_tables = Some(self.redefine_tables)
            .filter(|tables| !tables.is_empty())
            .map(SqlMigrationStep::RedefineTables);

        wrap_as_step(self.create_enums, SqlMigrationStep::CreateEnum)
            .chain(wrap_as_step(self.alter_enums, SqlMigrationStep::AlterEnum))
            .chain(wrap_as_step(self.drop_indexes, SqlMigrationStep::DropIndex))
            .chain(wrap_as_step(self.drop_foreign_keys, SqlMigrationStep::DropForeignKey))
            .chain(wrap_as_step(self.alter_tables, SqlMigrationStep::AlterTable))
            // Order matters: we must drop enums before we create tables,
            // because the new tables might be named the same as the dropped
            // enum, and that conflicts on postgres.
            .chain(wrap_as_step(self.drop_enums, SqlMigrationStep::DropEnum))
            .chain(wrap_as_step(self.create_tables, SqlMigrationStep::CreateTable))
            .chain(redefine_tables.into_iter())
            // Order matters: we must drop tables before we create indexes,
            // because on Postgres and SQLite, we may create indexes whose names
            // clash with the names of indexes on the dropped tables.
            .chain(wrap_as_step(self.drop_tables, SqlMigrationStep::DropTable))
            // Order matters: we must create indexes after ALTER TABLEs because the indexes can be
            // on fields that are dropped/created there.
            .chain(wrap_as_step(self.create_indexes, SqlMigrationStep::CreateIndex))
            // Order matters: this needs to come after create_indexes, because the foreign keys can depend on unique
            // indexes created there.
            .chain(wrap_as_step(self.add_foreign_keys, SqlMigrationStep::AddForeignKey))
            .chain(self.alter_indexes.into_iter().map(|idxs| SqlMigrationStep::AlterIndex {
                table: idxs.as_ref().map(|(table, _)| *table),
                index: idxs.as_ref().map(|(_, idx)| *idx),
            }))
            .chain(
                self.redefine_indexes
                    .into_iter()
                    .map(|idxs| SqlMigrationStep::RedefineIndex {
                        table: idxs.as_ref().map(|(table, _)| *table),
                        index: idxs.as_ref().map(|(_, idx)| *idx),
                    }),
            )
            .collect()
    }
}

impl<'schema> SqlSchemaDiffer<'schema> {
    fn diff_internal(&self) -> SqlSchemaDiff {
        let tables_to_redefine = self.flavour.tables_to_redefine(&self);
        let mut alter_indexes = self.alter_indexes(&tables_to_redefine);
        let redefine_indexes = if self.flavour.can_alter_index() {
            Vec::new()
        } else {
            std::mem::replace(&mut alter_indexes, Vec::new())
        };
        let (drop_tables, mut drop_foreign_keys) = self.drop_tables();
        self.drop_foreign_keys(&mut drop_foreign_keys, &tables_to_redefine);

        SqlSchemaDiff {
            add_foreign_keys: self.add_foreign_keys(&tables_to_redefine),
            drop_foreign_keys,
            drop_tables,
            create_tables: self.create_tables(),
            alter_tables: self.alter_tables(&tables_to_redefine),
            create_indexes: self.create_indexes(&tables_to_redefine),
            drop_indexes: self.drop_indexes(&tables_to_redefine),
            alter_indexes,
            redefine_indexes,
            create_enums: self.create_enums(),
            drop_enums: self.drop_enums(),
            alter_enums: self.alter_enums(),
            redefine_tables: self.redefine_tables(&tables_to_redefine),
            tables_to_redefine,
        }
    }

    fn create_tables(&self) -> Vec<CreateTable> {
        self.created_tables()
            .map(|created_table| CreateTable {
                table_index: created_table.table_index(),
            })
            .collect()
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

    fn alter_tables(&self, tables_to_redefine: &HashSet<String>) -> Vec<AlterTable> {
        self.table_pairs()
            .filter(|tables| !tables_to_redefine.contains(tables.next().name()))
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
            .collect()
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
        differ
            .created_primary_key()
            .filter(|pk| !pk.columns.is_empty())
            .map(|pk| TableChange::AddPrimaryKey {
                columns: pk.columns.clone(),
            })
    }

    fn drop_primary_key(differ: &TableDiffer<'_>) -> Option<TableChange> {
        differ.dropped_primary_key().map(|_pk| TableChange::DropPrimaryKey)
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
        let mut drop_indexes = Vec::new();

        for tables in self.table_pairs() {
            for index in tables.dropped_indexes() {
                // On MySQL, foreign keys automatically create indexes. These foreign-key-created
                // indexes should only be dropped as part of the foreign key.
                if self.flavour.should_skip_fk_indexes() && index::index_covers_fk(&tables.previous(), &index) {
                    continue;
                }

                drop_indexes.push(DropIndex {
                    table: tables.previous().name().to_owned(),
                    name: index.name().to_owned(),
                })
            }
        }

        // On SQLite, we will recreate indexes in the RedefineTables step,
        // because they are needed for implementing new foreign key constraints.
        if !tables_to_redefine.is_empty() && self.flavour.should_drop_indexes_from_dropped_tables() {
            drop_indexes.extend(self.dropped_tables().flat_map(|table| {
                table.indexes().map(move |index| DropIndex {
                    table: table.name().to_owned(),
                    name: index.name().to_owned(),
                })
            }))
        }

        drop_indexes
    }

    fn create_enums(&self) -> Vec<CreateEnum> {
        self.created_enums()
            .map(|r#enum| CreateEnum {
                enum_index: r#enum.enum_index(),
            })
            .collect()
    }

    fn drop_enums(&self) -> Vec<DropEnum> {
        self.dropped_enums()
            .map(|r#enum| DropEnum {
                enum_index: r#enum.enum_index(),
            })
            .collect()
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
                            changes.clone(),
                            type_change.map(|tc| match tc {
                                ColumnTypeChange::SafeCast => sql_migration::ColumnTypeChange::SafeCast,
                                ColumnTypeChange::RiskyCast => sql_migration::ColumnTypeChange::RiskyCast,
                                ColumnTypeChange::NotCastable => {
                                    unreachable!("ColumnTypeChange::NotCastable in redefine_tables")
                                }
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

    fn created_tables<'a>(&'a self) -> impl Iterator<Item = TableWalker<'a>> + 'a {
        self.next_tables()
            .filter(move |next_table| !self.schemas.previous().has_table(next_table.name()))
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
        table_name == MIGRATION_TABLE_NAME
            || table_name == "_prisma_migrations"
            || self.flavour.table_should_be_ignored(&table_name)
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
                previous.name() == next.name() && previous.column_type_family() == next.column_type_family()
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
