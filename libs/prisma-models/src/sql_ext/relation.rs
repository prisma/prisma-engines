use crate::{AsColumns, AsTable, ColumnIterator, Relation, RelationField, RelationLinkManifestation, RelationSide};
use quaint::ast::{Column, Table};

pub trait RelationExt {
    /// A helper function to decide actions based on the `Relation` type. Inline
    /// relation will return columns for updates, a relation table gives back `None`.
    fn columns_for_relation_side(&self, side: RelationSide) -> ColumnIterator;
    fn model_a_columns(&self) -> ColumnIterator;
    fn model_b_columns(&self) -> ColumnIterator;
}

pub trait RelationFieldExt {
    fn m2m_column_names(&self) -> Vec<String>;
    fn opposite_columns(&self, alias: bool) -> ColumnIterator;
    fn relation_columns(&self, alias: bool) -> ColumnIterator;

    // legacy single column unique
    fn relation_column(&self, alias: bool) -> Column<'static>;
}

impl RelationFieldExt for RelationField {
    fn m2m_column_names(&self) -> Vec<String> {
        let to_fields = &self.relation_info.to_fields;
        let prefix = if self.relation_side.is_a() { "B" } else { "A" };

        if to_fields.len() > 1 {
            to_fields
                .into_iter()
                .map(|to_field| format!("{}_{}", prefix, to_field))
                .collect()
        } else {
            vec![prefix.to_owned()]
        }
    }

    fn opposite_columns(&self, alias: bool) -> ColumnIterator {
        let cols = match self.relation_side {
            RelationSide::A => self.relation().model_b_columns(),
            RelationSide::B => self.relation().model_a_columns(),
        };

        if alias && !self.relation_is_inlined_in_child() {
            let count = cols.len();
            let inner = cols.into_iter().map(|col| col.table(Relation::TABLE_ALIAS));

            ColumnIterator::new(inner, count)
        } else {
            cols
        }
    }

    fn relation_columns(&self, alias: bool) -> ColumnIterator {
        let cols = match self.relation_side {
            RelationSide::A => self.relation().model_a_columns(),
            RelationSide::B => self.relation().model_b_columns(),
        };

        if alias && !self.relation_is_inlined_in_child() {
            let count = cols.len();
            let inner = cols.into_iter().map(|col| col.table(Relation::TABLE_ALIAS));

            ColumnIterator::new(inner, count)
        } else {
            cols
        }
    }

    fn relation_column(&self, alias: bool) -> Column<'static> {
        let mut col_iter = self.relation().columns_for_relation_side(self.relation_side);
        let col = col_iter.next().unwrap();

        if alias && !self.relation_is_inlined_in_child() {
            col.table(Relation::TABLE_ALIAS)
        } else {
            col
        }
    }
}

impl AsTable for Relation {
    /// The `Table` with the foreign keys are written. Can either be:
    ///
    /// - A separate table for many-to-many relations.
    /// - One of the model tables for one-to-many or one-to-one relations.
    /// - A separate relation table for all relations, if using the deprecated
    ///   data model syntax.
    fn as_table(&self) -> Table<'static> {
        match self.manifestation {
            RelationLinkManifestation::RelationTable(ref m) => {
                let db = self.model_a().internal_data_model().db_name.clone();
                (db, m.table.clone()).into()
            }
            RelationLinkManifestation::Inline(ref m) => self
                .internal_data_model()
                .find_model(&m.in_table_of_model_name)
                .unwrap()
                .as_table(),
        }
    }
}

impl RelationExt for Relation {
    fn columns_for_relation_side(&self, side: RelationSide) -> ColumnIterator {
        match side {
            RelationSide::A => self.model_a_columns(),
            RelationSide::B => self.model_b_columns(),
        }
    }

    #[allow(clippy::if_same_then_else)]
    fn model_a_columns(&self) -> ColumnIterator {
        use crate::RelationLinkManifestation::*;

        match self.manifestation {
            RelationTable(ref m) => ColumnIterator::from(vec![m.model_a_column.clone().into()]),
            Inline(ref m) => {
                let model_a = self.model_a();

                if self.is_self_relation() {
                    self.field_b().scalar_fields().as_columns()
                } else if m.in_table_of_model_name == model_a.name && !self.is_self_relation() {
                    let identifier = model_a.primary_identifier();
                    let count = identifier.scalar_length();

                    ColumnIterator::new(identifier.as_columns(), count)
                } else {
                    self.field_b().scalar_fields().as_columns()
                }
            }
        }
    }

    #[allow(clippy::if_same_then_else)]
    fn model_b_columns(&self) -> ColumnIterator {
        use crate::RelationLinkManifestation::*;

        match self.manifestation {
            RelationTable(ref m) => ColumnIterator::from(vec![m.model_b_column.clone().into()]),
            Inline(ref m) => {
                let model_b = self.model_b();

                if self.is_self_relation() {
                    let identifier = model_b.primary_identifier();
                    let count = identifier.scalar_length();

                    ColumnIterator::new(identifier.as_columns(), count)
                } else if m.in_table_of_model_name == model_b.name {
                    let identifier = model_b.primary_identifier();
                    let count = identifier.scalar_length();

                    ColumnIterator::new(identifier.as_columns(), count)
                } else {
                    self.field_a().scalar_fields().as_columns()
                }
            }
        }
    }
}
