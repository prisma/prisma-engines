mod column;
mod index;
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
}

impl SqlSchemaDiff {
    pub fn into_steps(self) -> Vec<SqlMigrationStep> {
        wrap_as_step(self.drop_indexes, SqlMigrationStep::DropIndex)
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

    fn is_mysql(&self) -> bool {
        match self.sql_family {
            SqlFamily::Mysql => true,
            _ => false,
        }
    }

    fn diff_internal(&self) -> SqlSchemaDiff {
        let alter_indexes: Vec<_> = self.alter_indexes();

        SqlSchemaDiff {
            add_foreign_keys: self.add_foreign_keys(),
            drop_tables: self.drop_tables(),
            create_tables: self.create_tables(),
            alter_tables: self.alter_tables(),
            create_indexes: self.create_indexes(&alter_indexes),
            drop_indexes: self.drop_indexes(&alter_indexes).collect(),
            alter_indexes,
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

    fn drop_indexes<'a>(&'a self, alter_indexes: &'a [AlterIndex]) -> impl Iterator<Item = DropIndex> + 'a {
        self.previous.tables.iter().flat_map(move |previous_table| {
            previous_table.indices.iter().filter_map(move |index| {
                // TODO: must diff index settings
                let next_index_opt = self
                    .next
                    .table(&previous_table.name)
                    .ok()
                    .and_then(|t| t.indices.iter().find(|i| i.name == index.name));

                let index_was_altered = alter_indexes.iter().any(|altered| altered.index_name == index.name);
                let index_was_dropped = next_index_opt.is_none() && !index_was_altered;

                if !index_was_dropped {
                    return None;
                }

                // If index covers PK, ignore it
                let index_covers_pk = match &previous_table.primary_key {
                    None => false,
                    Some(pk) => pk.columns == index.columns,
                };

                if index_covers_pk {
                    debug!(
                        "Not dropping index '{}' on table '{}' since it covers PK",
                        index.name, previous_table.name
                    );

                    return None;
                }

                // On MySQL, foreign keys automatically create indexes. These foreign-key-created
                // indexes should only be dropped as part of the foreign key.
                if self.is_mysql() && index::index_covers_fk(&previous_table, index) {
                    return None;
                }

                let drop = DropIndex {
                    table: previous_table.name.clone(),
                    name: index.name.clone(),
                };

                Some(drop)
            })
        })
    }

    /// An iterator over the tables that are present in both schemas.
    fn table_pairs<'a>(&'a self) -> impl Iterator<Item = TableDiffer<'schema>> + 'a {
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
        self.next.tables.iter().filter(move |next_table| {
            !self.previous.has_table(&next_table.name) && next_table.name != MIGRATION_TABLE_NAME
        })
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
