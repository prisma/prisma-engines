use crate::{AsTable, ColumnIterator, Relation, RelationField, RelationLinkManifestation, RelationSide};
use quaint::ast::Table;

pub trait RelationFieldExt {
    fn m2m_column_names(&self) -> Vec<String>;
    fn opposite_columns(&self) -> ColumnIterator;
    fn relation_columns(&self) -> ColumnIterator;
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

    fn opposite_columns(&self) -> ColumnIterator {
        use crate::RelationLinkManifestation::*;

        match (&self.relation().manifestation, &self.relation_side) {
            (RelationTable(ref m), RelationSide::A) => ColumnIterator::from(vec![m.model_b_column.clone().into()]),
            (RelationTable(ref m), RelationSide::B) => ColumnIterator::from(vec![m.model_a_column.clone().into()]),
            _ => unreachable!(),
        }
    }

    fn relation_columns(&self) -> ColumnIterator {
        use crate::RelationLinkManifestation::*;

        match (&self.relation().manifestation, &self.relation_side) {
            (RelationTable(ref m), RelationSide::A) => ColumnIterator::from(vec![m.model_a_column.clone().into()]),
            (RelationTable(ref m), RelationSide::B) => ColumnIterator::from(vec![m.model_b_column.clone().into()]),
            _ => unreachable!(),
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
