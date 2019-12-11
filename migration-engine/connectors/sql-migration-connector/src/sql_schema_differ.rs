mod column;
mod table;

use crate::*;
use log::debug;
use sql_schema_describer::*;
use table::TableDiffer;

const MIGRATION_TABLE_NAME: &str = "_Migration";

#[derive(Debug)]
pub struct SqlSchemaDiffer<'a> {
    previous: &'a SqlSchema,
    next: &'a SqlSchema,
}

#[derive(Debug, Clone)]
pub struct SqlSchemaDiff {
    pub drop_tables: Vec<DropTable>,
    pub create_tables: Vec<CreateTable>,
    pub alter_tables: Vec<AlterTable>,
    pub create_indexes: Vec<CreateIndex>,
    pub drop_indexes: Vec<DropIndex>,
    pub alter_indexes: Vec<AlterIndex>,
}

impl SqlSchemaDiff {
    pub fn into_steps(self) -> Vec<SqlMigrationStep> {
        wrap_as_step(self.drop_indexes, SqlMigrationStep::DropIndex)
            .chain(wrap_as_step(self.drop_tables, SqlMigrationStep::DropTable))
            .chain(wrap_as_step(self.create_tables, SqlMigrationStep::CreateTable))
            .chain(wrap_as_step(self.alter_tables, SqlMigrationStep::AlterTable))
            .chain(wrap_as_step(self.create_indexes, SqlMigrationStep::CreateIndex))
            .chain(wrap_as_step(self.alter_indexes, SqlMigrationStep::AlterIndex))
            .collect()
    }
}

impl<'a> SqlSchemaDiffer<'a> {
    pub fn diff(previous: &SqlSchema, next: &SqlSchema) -> SqlSchemaDiff {
        let differ = SqlSchemaDiffer { previous, next };
        differ.diff_internal()
    }

    fn diff_internal(&self) -> SqlSchemaDiff {
        let alter_indexes = self.alter_indexes();

        SqlSchemaDiff {
            drop_tables: self.drop_tables(),
            create_tables: self.create_tables(),
            alter_tables: self.alter_tables(),
            create_indexes: self.create_indexes(&alter_indexes),
            drop_indexes: self.drop_indexes(&alter_indexes),
            alter_indexes,
        }
    }

    fn create_tables(&self) -> Vec<CreateTable> {
        let mut result = Vec::new();
        for next_table in &self.next.tables {
            if !self.previous.has_table(&next_table.name) && next_table.name != MIGRATION_TABLE_NAME {
                let create = CreateTable {
                    table: next_table.clone(),
                };
                result.push(create);
            }
        }
        result
    }

    fn drop_tables(&self) -> Vec<DropTable> {
        let mut result = Vec::new();
        for previous_table in &self.previous.tables {
            if !self.next.has_table(&previous_table.name) && previous_table.name != MIGRATION_TABLE_NAME {
                let drop = DropTable {
                    name: previous_table.name.clone(),
                };
                result.push(drop);
            }
        }
        result
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

    fn drop_columns<'b>(differ: &'b TableDiffer<'b>) -> impl Iterator<Item = TableChange> + 'b {
        differ.dropped_columns().map(|column| {
            let change = DropColumn {
                name: column.name.clone(),
            };

            TableChange::DropColumn(change)
        })
    }

    fn add_columns<'b>(differ: &'b TableDiffer<'_>) -> impl Iterator<Item = TableChange> + 'b {
        differ.added_columns().map(|column| {
            let change = AddColumn { column: column.clone() };

            TableChange::AddColumn(change)
        })
    }

    fn alter_columns<'b>(table_differ: &'b TableDiffer<'_>) -> impl Iterator<Item = TableChange> + 'b {
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

    fn drop_foreign_keys<'b>(differ: &'b TableDiffer<'_>) -> impl Iterator<Item = TableChange> + 'b {
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

    fn create_indexes(&self, alter_indexes: &[AlterIndex]) -> Vec<CreateIndex> {
        let mut result = Vec::new();
        for next_table in &self.next.tables {
            for index in &next_table.indices {
                // TODO: must diff index settings
                let previous_index_opt = self
                    .previous
                    .table(&next_table.name)
                    .ok()
                    .and_then(|t| t.indices.iter().find(|i| i.name == index.name));
                let index_was_altered = alter_indexes.iter().any(|altered| altered.index_new_name == index.name);
                if previous_index_opt.is_none() && !index_was_altered {
                    let create = CreateIndex {
                        table: next_table.name.clone(),
                        index: index.clone(),
                    };
                    result.push(create);
                }
            }
        }
        result
    }

    fn drop_indexes(&self, alter_indexes: &[AlterIndex]) -> Vec<DropIndex> {
        let mut result = Vec::new();
        for previous_table in &self.previous.tables {
            for index in &previous_table.indices {
                // TODO: must diff index settings
                let next_index_opt = self
                    .next
                    .table(&previous_table.name)
                    .ok()
                    .and_then(|t| t.indices.iter().find(|i| i.name == index.name));
                let index_was_altered = alter_indexes.iter().any(|altered| altered.index_name == index.name);
                if next_index_opt.is_none() && !index_was_altered {
                    // If index covers PK, ignore it
                    let index_covers_pk = match &previous_table.primary_key {
                        None => false,
                        Some(pk) => pk.columns == index.columns,
                    };
                    if !index_covers_pk {
                        debug!("Dropping index '{}' on table '{}'", index.name, previous_table.name);
                        let drop = DropIndex {
                            table: previous_table.name.clone(),
                            name: index.name.clone(),
                        };
                        result.push(drop);
                    } else {
                        debug!(
                            "Not dropping index '{}' on table '{}' since it covers PK",
                            index.name, previous_table.name
                        );
                    }
                }
            }
        }
        result
    }

    /// An iterator over the tables that are present in both schemas.
    fn table_pairs<'b>(&'b self) -> impl Iterator<Item = TableDiffer<'b>> {
        self.previous.tables.iter().filter_map(move |previous_table| {
            self.next
                .tables
                .iter()
                .find(move |next_table| next_table.name == previous_table.name)
                .map(move |next_table| TableDiffer {
                    previous: previous_table,
                    next: next_table,
                })
        })
    }

    fn alter_indexes(&self) -> Vec<AlterIndex> {
        self.table_pairs()
            .flat_map(|differ| {
                let next_table = differ.next;
                differ
                    .previous
                    .indices
                    .iter()
                    .filter_map(move |previous_index| {
                        next_table
                            .indices
                            .iter()
                            .find(|next_index| {
                                indexes_are_equivalent(previous_index, next_index)
                                    && previous_index.name != next_index.name
                            })
                            .map(|renamed_index| (previous_index, renamed_index))
                    })
                    .map(move |(previous_index, renamed_index)| AlterIndex {
                        index_name: previous_index.name.clone(),
                        index_new_name: renamed_index.name.clone(),
                        table: differ.next.name.clone(),
                    })
            })
            .collect()
    }
}

/// Compare two SQL indexes and return whether they only differ by name or type.
fn indexes_are_equivalent(first: &Index, second: &Index) -> bool {
    first.columns == second.columns && first.tpe == second.tpe
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
