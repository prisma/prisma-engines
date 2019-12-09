use super::column::ColumnDiffer;
use sql_schema_describer::{Column, ForeignKey, Table};

pub(crate) struct TableDiffer<'a> {
    pub(crate) previous: &'a Table,
    pub(crate) next: &'a Table,
}

impl<'a> TableDiffer<'a> {
    pub(crate) fn column_pairs(&'a self) -> impl Iterator<Item = ColumnDiffer<'a>> {
        self.previous_columns()
            .filter_map(move |previous_column| {
                self.next_columns()
                    .find(|next_column| columns_match(previous_column, next_column))
                    .map(|next_column| (previous_column, next_column))
            })
            .map(|(previous, next)| ColumnDiffer { previous, next })
    }

    pub(crate) fn dropped_columns(&'a self) -> impl Iterator<Item = &'a Column> + 'a {
        self.previous_columns().filter(move |previous_column| {
            self.next_columns()
                .find(|next_column| columns_match(previous_column, next_column))
                .is_none()
        })
    }

    pub(crate) fn added_columns(&self) -> impl Iterator<Item = &Column> {
        self.next_columns().filter(move |next_column| {
            self.previous_columns()
                .find(|previous_column| columns_match(previous_column, next_column))
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

    fn previous_columns(&self) -> impl Iterator<Item = &Column> {
        self.previous.columns.iter()
    }

    fn next_columns(&self) -> impl Iterator<Item = &Column> {
        self.next.columns.iter()
    }

    fn previous_foreign_keys(&self) -> impl Iterator<Item = &ForeignKey> {
        self.previous.foreign_keys.iter()
    }

    fn next_foreign_keys(&self) -> impl Iterator<Item = &ForeignKey> {
        self.next.foreign_keys.iter()
    }
}

fn columns_match(a: &Column, b: &Column) -> bool {
    a.name == b.name
}
