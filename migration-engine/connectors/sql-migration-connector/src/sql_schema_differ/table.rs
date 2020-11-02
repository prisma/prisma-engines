use super::column::ColumnDiffer;
use crate::{database_info::DatabaseInfo, flavour::SqlFlavour};
use sql_schema_describer::{
    walkers::{ColumnWalker, ForeignKeyWalker, IndexWalker, TableWalker},
    PrimaryKey,
};

pub(crate) struct TableDiffer<'a> {
    pub(crate) database_info: &'a DatabaseInfo,
    pub(crate) flavour: &'a dyn SqlFlavour,
    pub(crate) previous: TableWalker<'a>,
    pub(crate) next: TableWalker<'a>,
}

impl<'schema> TableDiffer<'schema> {
    pub(crate) fn column_pairs<'a>(&'a self) -> impl Iterator<Item = ColumnDiffer<'schema>> + 'a {
        self.previous_columns()
            .filter_map(move |previous_column| {
                self.next_columns()
                    .find(|next_column| columns_match(&previous_column, next_column))
                    .map(|next_column| (previous_column, next_column))
            })
            .map(move |(previous, next)| ColumnDiffer {
                database_info: self.database_info,
                flavour: self.flavour,
                previous,
                next,
            })
    }

    pub(crate) fn dropped_columns<'a>(&'a self) -> impl Iterator<Item = ColumnWalker<'schema>> + 'a {
        self.previous_columns().filter(move |previous_column| {
            self.next_columns()
                .find(|next_column| columns_match(previous_column, next_column))
                .is_none()
        })
    }

    pub(crate) fn added_columns<'a>(&'a self) -> impl Iterator<Item = ColumnWalker<'schema>> + 'a {
        self.next_columns().filter(move |next_column| {
            self.previous_columns()
                .find(|previous_column| columns_match(previous_column, next_column))
                .is_none()
        })
    }

    pub(crate) fn created_foreign_keys<'a>(&'a self) -> impl Iterator<Item = ForeignKeyWalker<'schema>> + 'a {
        self.next_foreign_keys().filter(move |next_fk| {
            self.previous_foreign_keys()
                .find(|previous_fk| super::foreign_keys_match(previous_fk, next_fk))
                .is_none()
        })
    }

    pub(crate) fn dropped_foreign_keys<'a>(&'a self) -> impl Iterator<Item = ForeignKeyWalker<'schema>> + 'a {
        self.previous_foreign_keys().filter(move |previous_fk| {
            self.next_foreign_keys()
                .find(|next_fk| super::foreign_keys_match(previous_fk, next_fk))
                .is_none()
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

    pub(crate) fn index_pairs<'a>(&'a self) -> impl Iterator<Item = (IndexWalker<'schema>, IndexWalker<'schema>)> + 'a {
        self.previous_indexes().filter_map(move |previous_index| {
            self.next_indexes()
                .find(|next_index| indexes_match(&previous_index, next_index))
                .map(|renamed_index| (previous_index, renamed_index))
        })
    }

    /// The primary key present in `next` but not `previous`, if applicable.
    pub(crate) fn created_primary_key(&self) -> Option<&'schema PrimaryKey> {
        match (self.previous.primary_key(), self.next.primary_key()) {
            (None, Some(pk)) => Some(pk),
            (Some(previous_pk), Some(next_pk)) if previous_pk.columns != next_pk.columns => Some(next_pk),
            (Some(previous_pk), Some(next_pk)) => {
                if self.primary_key_column_changed(previous_pk) {
                    Some(next_pk)
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    /// The primary key present in `previous` but not `next`, if applicable.
    pub(crate) fn dropped_primary_key(&self) -> Option<&'schema PrimaryKey> {
        match (self.previous.primary_key(), self.next.primary_key()) {
            (Some(pk), None) => Some(pk),
            (Some(previous_pk), Some(next_pk)) if previous_pk.columns != next_pk.columns => Some(previous_pk),
            (Some(previous_pk), Some(_next_pk)) => {
                if self.primary_key_column_changed(previous_pk) {
                    Some(previous_pk)
                } else {
                    None
                }
            }
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
            .any(|columns| columns.all_changes().0.type_changed())
    }

    fn previous_columns<'a>(&'a self) -> impl Iterator<Item = ColumnWalker<'schema>> + 'a {
        self.previous.columns()
    }

    fn next_columns<'a>(&'a self) -> impl Iterator<Item = ColumnWalker<'schema>> + 'a {
        self.next.columns()
    }

    fn previous_foreign_keys<'a>(&'a self) -> impl Iterator<Item = ForeignKeyWalker<'schema>> + 'a {
        self.previous.foreign_keys()
    }

    fn next_foreign_keys<'a>(&'a self) -> impl Iterator<Item = ForeignKeyWalker<'schema>> + 'a {
        self.next.foreign_keys()
    }

    fn previous_indexes<'a>(&'a self) -> impl Iterator<Item = IndexWalker<'schema>> + 'a {
        self.previous.indexes()
    }

    fn next_indexes<'a>(&'a self) -> impl Iterator<Item = IndexWalker<'schema>> + 'a {
        self.next.indexes()
    }
}

pub(crate) fn columns_match(a: &ColumnWalker<'_>, b: &ColumnWalker<'_>) -> bool {
    a.name() == b.name()
}

/// Compare two SQL indexes and return whether they only differ by name.
fn indexes_match(first: &IndexWalker<'_>, second: &IndexWalker<'_>) -> bool {
    first.column_names() == second.column_names() && first.index_type() == second.index_type()
}
