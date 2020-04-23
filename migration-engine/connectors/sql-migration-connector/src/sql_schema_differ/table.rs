use super::column::ColumnDiffer;
use sql_schema_describer::{Column, ForeignKey, Index, Table};

pub(crate) struct TableDiffer<'a> {
    pub(crate) diffing_options: &'a super::DiffingOptions,
    pub(crate) previous: &'a Table,
    pub(crate) next: &'a Table,
}

impl<'schema> TableDiffer<'schema> {
    pub(crate) fn column_pairs<'a>(&'a self) -> impl Iterator<Item = ColumnDiffer<'schema>> + 'a {
        self.previous_columns()
            .filter_map(move |previous_column| {
                self.next_columns()
                    .find(|next_column| columns_match(previous_column, next_column))
                    .map(|next_column| (previous_column, next_column))
            })
            .map(move |(previous, next)| ColumnDiffer {
                diffing_options: self.diffing_options,
                previous,
                next,
            })
    }

    pub(crate) fn dropped_columns<'a>(&'a self) -> impl Iterator<Item = &'schema Column> + 'a {
        self.previous_columns().filter(move |previous_column| {
            self.next_columns()
                .find(|next_column| columns_match(previous_column, next_column))
                .is_none()
        })
    }

    pub(crate) fn added_columns<'a>(&'a self) -> impl Iterator<Item = &'schema Column> + 'a {
        self.next_columns().filter(move |next_column| {
            self.previous_columns()
                .find(|previous_column| columns_match(previous_column, next_column))
                .is_none()
        })
    }

    pub(crate) fn created_foreign_keys(&self) -> impl Iterator<Item = &ForeignKey> {
        self.next_foreign_keys().filter(move |next_fk| {
            self.previous_foreign_keys()
                .find(|previous_fk| super::foreign_keys_match(previous_fk, next_fk))
                .is_none()
        })
    }

    pub(crate) fn dropped_foreign_keys(&self) -> impl Iterator<Item = &ForeignKey> {
        self.previous_foreign_keys().filter(move |previous_fk| {
            self.next_foreign_keys()
                .find(|next_fk| super::foreign_keys_match(previous_fk, next_fk))
                .is_none()
        })
    }

    pub(crate) fn created_indexes<'a>(&'a self) -> impl Iterator<Item = &'schema Index> + 'a {
        self.next_indexes().filter(move |next_index| {
            !self
                .previous_indexes()
                .any(move |previous_index| indexes_match(previous_index, next_index))
        })
    }

    pub(crate) fn dropped_indexes<'a>(&'a self) -> impl Iterator<Item = &'schema Index> + 'a {
        self.previous_indexes().filter(move |previous_index| {
            !self
                .next_indexes()
                .any(|next_index| indexes_match(previous_index, next_index))
        })
    }

    pub(crate) fn index_pairs<'a>(&'a self) -> impl Iterator<Item = (&'schema Index, &'schema Index)> + 'a {
        self.previous_indexes().filter_map(move |previous_index| {
            self.next_indexes()
                .find(|next_index| indexes_match(previous_index, next_index) && previous_index.name != next_index.name)
                .map(|renamed_index| (previous_index, renamed_index))
        })
    }

    fn previous_columns(&self) -> impl Iterator<Item = &'schema Column> {
        self.previous.columns.iter()
    }

    fn next_columns(&self) -> impl Iterator<Item = &'schema Column> {
        self.next.columns.iter()
    }

    fn previous_foreign_keys(&self) -> impl Iterator<Item = &ForeignKey> {
        self.previous.foreign_keys.iter()
    }

    fn next_foreign_keys(&self) -> impl Iterator<Item = &ForeignKey> {
        self.next.foreign_keys.iter()
    }

    fn previous_indexes<'a>(&'a self) -> impl Iterator<Item = &'schema Index> + 'a {
        self.previous.indices.iter()
    }

    fn next_indexes<'a>(&'a self) -> impl Iterator<Item = &'schema Index> + 'a {
        self.next.indices.iter()
    }
}

fn columns_match(a: &Column, b: &Column) -> bool {
    a.name == b.name
}

/// Compare two SQL indexes and return whether they only differ by name.
fn indexes_match(first: &Index, second: &Index) -> bool {
    first.columns == second.columns && first.tpe == second.tpe
}
