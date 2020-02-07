mod column;
mod index;
mod table;

pub(crate) use column::{ColumnChange, ColumnDiffer};
pub(crate) use table::TableDiffer;

use crate::*;
use sql_schema_describer::*;
use tracing::debug;

const MIGRATION_TABLE_NAME: &str = "_Migration";

#[derive(Debug)]
pub struct SqlSchemaDiffer<'a> {
    previous: &'a SqlSchema,
    next: &'a SqlSchema,
    sql_family: SqlFamily,
}

#[derive(Debug, Clone)]
pub struct SqlSchemaDiff {
    pub add_foreign_keys: Vec<AddForeignKey>,
    pub drop_tables: Vec<DropTable>,
    pub create_tables: Vec<CreateTable>,
    pub alter_tables: Vec<AlterTable>,
    pub create_indexes: Vec<CreateIndex>,
    pub drop_indexes: Vec<DropIndex>,
    pub alter_indexes: Vec<AlterIndex>,
    pub create_enums: Vec<CreateEnum>,
    pub drop_enums: Vec<DropEnum>,
    pub alter_enums: Vec<AlterEnum>,
}

impl SqlSchemaDiff {
    pub fn into_steps(self) -> Vec<SqlMigrationStep> {
        wrap_as_step(self.create_enums, SqlMigrationStep::CreateEnum)
            .chain(wrap_as_step(self.drop_indexes, SqlMigrationStep::DropIndex))
            // Order matters: we must create tables before `alter_table`s because we could
            // be adding foreign keys to the new tables there.
            .chain(wrap_as_step(self.create_tables, SqlMigrationStep::CreateTable))
            // Order matters: we must run `alter table`s before `drop`s because we want to
            // drop foreign keys before the tables they are pointing to.
            .chain(wrap_as_step(self.alter_tables, SqlMigrationStep::AlterTable))
            // Order matters: we must create indexes after ALTER TABLEs because the indexes can be on fields that
            // are dropped/created there.
            .chain(wrap_as_step(self.create_indexes, SqlMigrationStep::CreateIndex))
            // Order matters: this needs to come after create_indexes, because the foreign keys can depend on unique
            // indexes created there.
            .chain(wrap_as_step(self.add_foreign_keys, SqlMigrationStep::AddForeignKey))
            .chain(wrap_as_step(self.drop_tables, SqlMigrationStep::DropTable))
            .chain(wrap_as_step(self.drop_enums, SqlMigrationStep::DropEnum))
            .chain(wrap_as_step(self.alter_indexes, SqlMigrationStep::AlterIndex))
            .collect()
    }
}

impl<'schema> SqlSchemaDiffer<'schema> {
    pub fn diff(previous: &SqlSchema, next: &SqlSchema, sql_family: SqlFamily) -> SqlSchemaDiff {
        let differ = SqlSchemaDiffer {
            previous,
            next,
            sql_family,
        };
        differ.diff_internal()
    }

    fn diff_internal(&self) -> SqlSchemaDiff {
        let alter_indexes: Vec<_> = self.alter_indexes();

        SqlSchemaDiff {
            add_foreign_keys: self.add_foreign_keys(),
            drop_tables: self.drop_tables(),
            create_tables: self.create_tables(),
            alter_tables: self.alter_tables(),
            create_indexes: self.create_indexes(),
            drop_indexes: self.drop_indexes(),
            alter_indexes,
            create_enums: self.create_enums(),
            drop_enums: self.drop_enums(),
            alter_enums: self.alter_enums(),
        }
    }

    fn create_tables(&self) -> Vec<CreateTable> {
        self.created_tables()
            .map(|created_table| CreateTable {
                table: created_table.clone(),
            })
            .collect()
    }

    fn drop_tables(&self) -> Vec<DropTable> {
        self.dropped_tables()
            .map(|dropped_table| DropTable {
                name: dropped_table.name.clone(),
            })
            .collect()
    }

    fn add_foreign_keys(&self) -> Vec<AddForeignKey> {
        let mut add_foreign_keys = Vec::new();

        push_foreign_keys_from_created_tables(&mut add_foreign_keys, self.created_tables());
        push_created_foreign_keys(&mut add_foreign_keys, self.table_pairs());

        add_foreign_keys
    }

    fn alter_tables(&self) -> Vec<AlterTable> {
        // TODO: this does not diff primary key columns yet
        let mut result = Vec::new();
        for previous_table in &self.previous.tables {
            if let Ok(next_table) = self.next.table(&previous_table.name) {
                let differ = TableDiffer {
                    previous: &previous_table,
                    next: &next_table,
                };

                let changes: Vec<TableChange> = Self::drop_foreign_keys(&differ)
                    .chain(Self::drop_columns(&differ))
                    .chain(Self::add_columns(&differ))
                    .chain(Self::alter_columns(&differ))
                    .collect();

                if !changes.is_empty() {
                    let update = AlterTable {
                        table: next_table.clone(),
                        changes,
                    };
                    result.push(update);
                }
            }
        }

        result
    }

    fn drop_columns<'a>(differ: &'a TableDiffer<'schema>) -> impl Iterator<Item = TableChange> + 'a {
        differ.dropped_columns().map(|column| {
            let change = DropColumn {
                name: column.name.clone(),
            };

            TableChange::DropColumn(change)
        })
    }

    fn add_columns<'a>(differ: &'a TableDiffer<'schema>) -> impl Iterator<Item = TableChange> + 'a {
        differ.added_columns().map(move |column| {
            let change = AddColumn { column: column.clone() };

            TableChange::AddColumn(change)
        })
    }

    fn alter_columns<'a>(table_differ: &'a TableDiffer<'schema>) -> impl Iterator<Item = TableChange> + 'a {
        table_differ.column_pairs().filter_map(move |column_differ| {
            let previous_fk = table_differ
                .previous
                .foreign_key_for_column(&column_differ.previous.name);

            let next_fk = table_differ.next.foreign_key_for_column(&column_differ.next.name);

            if column_differ.differs_in_something() || foreign_key_changed(previous_fk, next_fk) {
                let change = AlterColumn {
                    name: column_differ.previous.name.clone(),
                    column: column_differ.next.clone(),
                };

                return Some(TableChange::AlterColumn(change));
            }

            None
        })
    }

    fn drop_foreign_keys<'a>(differ: &'a TableDiffer<'schema>) -> impl Iterator<Item = TableChange> + 'a {
        differ
            .dropped_foreign_keys()
            .filter_map(|foreign_key| foreign_key.constraint_name.as_ref())
            .map(move |dropped_foreign_key_name| {
                debug!(
                    "Dropping foreign key '{}' on table '{}'",
                    &dropped_foreign_key_name, &differ.previous.name
                );
                let drop_step = DropForeignKey {
                    constraint_name: dropped_foreign_key_name.clone(),
                };
                TableChange::DropForeignKey(drop_step)
            })
    }

    fn create_indexes(&self) -> Vec<CreateIndex> {
        let mut steps = Vec::new();

        for table in self.created_tables() {
            for index in &table.indices {
                let create = CreateIndex {
                    table: table.name.clone(),
                    index: index.clone(),
                };

                steps.push(create)
            }
        }

        for tables in self.table_pairs() {
            for index in tables.created_indexes() {
                let create = CreateIndex {
                    table: tables.next.name.clone(),
                    index: index.clone(),
                };

                steps.push(create)
            }
        }

        steps
    }

    fn drop_indexes(&self) -> Vec<DropIndex> {
        let mut drop_indexes = Vec::new();

        for tables in self.table_pairs() {
            for index in tables.dropped_indexes() {
                // On MySQL, foreign keys automatically create indexes. These foreign-key-created
                // indexes should only be dropped as part of the foreign key.
                if self.sql_family.is_mysql() && index::index_covers_fk(&tables.previous, index) {
                    continue;
                }
                drop_indexes.push(DropIndex {
                    table: tables.previous.name.clone(),
                    name: index.name.clone(),
                })
            }
        }

        drop_indexes
    }

    fn create_enums(&self) -> Vec<CreateEnum> {
        self.created_enums()
            .map(|r#enum| CreateEnum {
                name: r#enum.name.clone(),
                variants: r#enum.values.clone(),
            })
            .collect()
    }

    fn drop_enums(&self) -> Vec<DropEnum> {
        self.dropped_enums()
            .map(|r#enum| DropEnum {
                name: r#enum.name.clone(),
            })
            .collect()
    }

    fn alter_enums(&self) -> Vec<AlterEnum> {
        Vec::new()
    }

    /// An iterator over the tables that are present in both schemas.
    fn table_pairs<'a>(&'a self) -> impl Iterator<Item = TableDiffer<'schema>> + 'a
    where
        'schema: 'a,
    {
        self.previous.tables.iter().filter_map(move |previous_table| {
            self.next
                .tables
                .iter()
                .find(move |next_table| tables_match(previous_table, next_table))
                .map(move |next_table| TableDiffer {
                    previous: previous_table,
                    next: next_table,
                })
        })
    }

    fn alter_indexes<'a>(&'a self) -> Vec<AlterIndex> {
        let mut alter_indexes = Vec::new();
        self.table_pairs().for_each(|differ| {
            differ.index_pairs().for_each(|(previous_index, renamed_index)| {
                alter_indexes.push(AlterIndex {
                    index_name: previous_index.name.clone(),
                    index_new_name: renamed_index.name.clone(),
                    table: differ.next.name.clone(),
                })
            })
        });

        alter_indexes
    }

    fn created_tables<'a>(&'a self) -> impl Iterator<Item = &'a Table> + 'a {
        self.next_tables()
            .filter(move |next_table| !self.previous.has_table(&next_table.name))
    }

    fn dropped_tables(&self) -> impl Iterator<Item = &Table> {
        self.previous_tables().filter(move |previous_table| {
            !self
                .next_tables()
                .any(|next_table| tables_match(previous_table, next_table))
        })
    }

    fn previous_tables(&self) -> impl Iterator<Item = &Table> {
        self.previous
            .tables
            .iter()
            .filter(|table| table.name != MIGRATION_TABLE_NAME)
    }

    fn next_tables(&self) -> impl Iterator<Item = &Table> {
        self.next
            .tables
            .iter()
            .filter(|table| table.name != MIGRATION_TABLE_NAME)
    }

    fn created_enums(&self) -> impl Iterator<Item = &Enum> {
        self.next_enums()
            .filter(move |next| !self.previous_enums().any(|previous| enums_match(previous, next)))
    }

    fn dropped_enums(&self) -> impl Iterator<Item = &Enum> {
        self.previous_enums()
            .filter(move |previous| !self.next_enums().any(|next| enums_match(previous, next)))
    }

    fn previous_enums(&self) -> impl Iterator<Item = &Enum> {
        self.previous.enums.iter()
    }

    fn next_enums(&self) -> impl Iterator<Item = &Enum> {
        self.next.enums.iter()
    }
}

fn push_created_foreign_keys<'a, 'schema>(
    added_foreign_keys: &mut Vec<AddForeignKey>,
    table_pairs: impl Iterator<Item = TableDiffer<'schema>>,
) {
    table_pairs.for_each(|differ| {
        added_foreign_keys.extend(differ.created_foreign_keys().map(|created_fk| AddForeignKey {
            table: differ.next.name.clone(),
            foreign_key: created_fk.clone(),
        }))
    })
}

fn push_foreign_keys_from_created_tables<'a>(
    steps: &mut Vec<AddForeignKey>,
    created_tables: impl Iterator<Item = &'a Table>,
) {
    for table in created_tables {
        steps.extend(table.foreign_keys.iter().map(|fk| AddForeignKey {
            table: table.name.clone(),
            foreign_key: fk.clone(),
        }));
    }
}

/// Compare two [ForeignKey](/sql-schema-describer/struct.ForeignKey.html)s and return whether a
/// migration needs to be applied.
fn foreign_key_changed(previous: Option<&ForeignKey>, next: Option<&ForeignKey>) -> bool {
    match (previous, next) {
        (None, None) => false,
        (Some(previous), Some(next)) => !foreign_keys_match(previous, next),
        _ => true,
    }
}

/// Compare two [ForeignKey](/sql-schema-describer/struct.ForeignKey.html)s and return whether they
/// should be considered equivalent for schema diffing purposes.
fn foreign_keys_match(previous: &ForeignKey, next: &ForeignKey) -> bool {
    previous.referenced_table == next.referenced_table
        && previous.referenced_columns == next.referenced_columns
        && previous.columns == next.columns
        && previous.on_delete_action == next.on_delete_action
}

fn tables_match(previous: &Table, next: &Table) -> bool {
    previous.name == next.name
}

fn enums_match(previous: &Enum, next: &Enum) -> bool {
    previous.name == next.name
}
