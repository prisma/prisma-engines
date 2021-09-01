use super::{differ_database::DifferDatabase, foreign_keys_match};
use crate::pair::Pair;
use sql_schema_describer::{
    walkers::{ColumnWalker, ForeignKeyWalker, IndexWalker, TableWalker},
    PrimaryKey,
};

pub(crate) struct TableDiffer<'a, 'b> {
    pub(crate) tables: Pair<TableWalker<'a>>,
    pub(crate) db: &'b DifferDatabase<'a>,
}

impl<'schema, 'b> TableDiffer<'schema, 'b> {
    pub(crate) fn column_pairs(&self) -> impl Iterator<Item = Pair<ColumnWalker<'schema>>> + '_ {
        self.db
            .column_pairs(self.tables.map(|t| t.table_id()))
            .map(move |colids| self.tables.columns(&colids))
    }

    pub(crate) fn any_column_changed(&self) -> bool {
        self.column_pairs()
            .any(|col| self.db.column_changes_for_walkers(col).differs_in_something())
    }

    pub(crate) fn dropped_columns<'a>(&'a self) -> impl Iterator<Item = ColumnWalker<'schema>> + 'a {
        self.db
            .dropped_columns(self.tables.map(|t| t.table_id()))
            .map(move |idx| self.tables.previous().column_at(idx))
    }

    pub(crate) fn added_columns<'a>(&'a self) -> impl Iterator<Item = ColumnWalker<'schema>> + 'a {
        self.db
            .created_columns(self.tables.map(|t| t.table_id()))
            .map(move |idx| self.tables.next().column_at(idx))
    }

    pub(crate) fn created_foreign_keys<'a>(&'a self) -> impl Iterator<Item = ForeignKeyWalker<'schema>> + 'a {
        self.next_foreign_keys().filter(move |next_fk| {
            !self
                .previous_foreign_keys()
                .any(|previous_fk| super::foreign_keys_match(Pair::new(&previous_fk, next_fk), self.db))
        })
    }

    pub(crate) fn dropped_foreign_keys<'a>(&'a self) -> impl Iterator<Item = ForeignKeyWalker<'schema>> + 'a {
        self.previous_foreign_keys().filter(move |previous_fk| {
            !self
                .next_foreign_keys()
                .any(|next_fk| super::foreign_keys_match(Pair::new(previous_fk, &next_fk), self.db))
        })
    }

    pub(crate) fn created_indexes<'a>(&'a self) -> impl Iterator<Item = IndexWalker<'schema>> + 'a {
        self.next_indexes().filter(move |next_index| {
            !self
                .previous_indexes()
                .any(move |previous_index| indexes_match(&previous_index, next_index))
        })
    }

    pub(crate) fn dropped_indexes<'a>(&'a self) -> impl Iterator<Item = IndexWalker<'schema>> + 'a {
        self.previous_indexes().filter(move |previous_index| {
            !self
                .next_indexes()
                .any(|next_index| indexes_match(previous_index, &next_index))
        })
    }

    pub(crate) fn foreign_key_pairs(&self) -> impl Iterator<Item = Pair<ForeignKeyWalker<'schema>>> + '_ {
        self.previous_foreign_keys().filter_map(move |previous_fk| {
            self.next_foreign_keys()
                .find(move |next_fk| foreign_keys_match(Pair::new(&previous_fk, next_fk), self.db))
                .map(move |next_fk| Pair::new(previous_fk, next_fk))
        })
    }

    pub(crate) fn index_pairs<'a>(&'a self) -> impl Iterator<Item = Pair<IndexWalker<'schema>>> + 'a {
        let singular_indexes = self.previous_indexes().filter(move |left| {
            // Renaming an index in a situation where we have multiple indexes
            // with the same columns, but a different name, is highly unstable.
            // We do not rename them for now.
            let number_of_identical_indexes = self
                .previous_indexes()
                .filter(|right| left.column_names() == right.column_names() && left.index_type() == right.index_type())
                .count();

            number_of_identical_indexes == 1
        });

        singular_indexes.filter_map(move |previous_index| {
            self.next_indexes()
                .find(|next_index| indexes_match(&previous_index, next_index))
                .map(|renamed_index| Pair::new(previous_index, renamed_index))
        })
    }

    /// The primary key present in `next` but not `previous`, if applicable.
    pub(crate) fn created_primary_key(&self) -> Option<&'schema PrimaryKey> {
        match self.tables.as_ref().map(|t| t.primary_key()).as_tuple() {
            (None, Some(pk)) => Some(pk),
            (Some(previous_pk), Some(next_pk)) if previous_pk.columns != next_pk.columns => Some(next_pk),
            (Some(previous_pk), Some(next_pk)) => self.primary_key_column_changed(previous_pk).then(|| *next_pk),
            _ => None,
        }
    }

    /// The primary key present in `previous` but not `next`, if applicable.
    pub(crate) fn dropped_primary_key(&self) -> Option<&'schema PrimaryKey> {
        match self.tables.as_ref().map(|t| t.primary_key()).as_tuple() {
            (Some(pk), None) => Some(pk),
            (Some(previous_pk), Some(next_pk)) if previous_pk.columns != next_pk.columns => Some(previous_pk),
            (Some(previous_pk), Some(_next_pk)) => self.primary_key_column_changed(previous_pk).then(|| *previous_pk),
            _ => None,
        }
    }

    /// Returns true if any of the columns of the primary key changed type.
    fn primary_key_column_changed(&self, previous_pk: &PrimaryKey) -> bool {
        self.column_pairs()
            .filter(|columns| {
                previous_pk
                    .columns
                    .iter()
                    .any(|pk_col| pk_col == columns.previous.name())
            })
            .any(|columns| self.db.column_changes_for_walkers(columns).type_changed())
    }

    fn previous_foreign_keys<'a>(&'a self) -> impl Iterator<Item = ForeignKeyWalker<'schema>> + 'a {
        self.previous().foreign_keys()
    }

    fn next_foreign_keys<'a>(&'a self) -> impl Iterator<Item = ForeignKeyWalker<'schema>> + 'a {
        self.next().foreign_keys()
    }

    fn previous_indexes(&self) -> impl Iterator<Item = IndexWalker<'schema>> {
        self.previous().indexes()
    }

    fn next_indexes(&self) -> impl Iterator<Item = IndexWalker<'schema>> {
        self.next().indexes()
    }

    pub(super) fn previous(&self) -> &TableWalker<'schema> {
        self.tables.previous()
    }

    pub(super) fn next(&self) -> &TableWalker<'schema> {
        self.tables.next()
    }
}

/// Compare two SQL indexes and return whether they only differ by name.
fn indexes_match(first: &IndexWalker<'_>, second: &IndexWalker<'_>) -> bool {
    first.column_names() == second.column_names() && first.index_type() == second.index_type()
}
