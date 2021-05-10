use crate::pair::Pair;
use sql_schema_describer::{
    walkers::{IndexWalker, TableWalker},
    PrimaryKey,
};

impl<'a> Pair<TableWalker<'a>> {
    pub(crate) fn index_pairs(&self) -> impl Iterator<Item = Pair<IndexWalker<'a>>> + '_ {
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
    pub(crate) fn created_primary_key(&self) -> Option<&'a PrimaryKey> {
        match self.as_ref().map(|t| t.primary_key()).as_tuple() {
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
    pub(crate) fn dropped_primary_key(&self) -> Option<&'a PrimaryKey> {
        match self.as_ref().map(|t| t.primary_key()).as_tuple() {
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

    pub(crate) fn table_indexes(&self) -> Pair<usize> {
        self.map(|t| t.table_index())
    }
}
