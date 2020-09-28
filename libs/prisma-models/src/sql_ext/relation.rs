use crate::sql_ext::column::AsColumns;
use crate::RelationLinkManifestation::*;
use crate::{AsTable, ColumnIterator, Relation, RelationField, RelationLinkManifestation, RelationSide};
use quaint::{ast::Table, prelude::Column};

pub trait RelationFieldExt {
    fn m2m_columns(&self) -> Vec<Column<'static>>;
    fn join_columns(&self) -> ColumnIterator;
    fn identifier_columns(&self) -> ColumnIterator;
    fn as_table(&self) -> Table<'static>;
}

impl RelationFieldExt for RelationField {
    fn m2m_columns(&self) -> Vec<Column<'static>> {
        let to_fields = &self.relation_info.to_fields;
        let prefix = if self.relation_side.is_a() { "B" } else { "A" };

        if to_fields.len() > 1 {
            to_fields
                .into_iter()
                .map(|to_field| format!("{}_{}", prefix, to_field))
                .map(|name| Column::from(name).table(self.as_table()))
                .collect()
        } else {
            vec![Column::from(prefix).table(self.as_table())]
        }
    }

    fn join_columns(&self) -> ColumnIterator {
        match (&self.relation().manifestation, &self.relation_side) {
            (RelationTable(ref m), RelationSide::A) => ColumnIterator::from(vec![m.model_b_column.clone().into()]),
            (RelationTable(ref m), RelationSide::B) => ColumnIterator::from(vec![m.model_a_column.clone().into()]),
            _ => self.linking_fields().as_columns(),
        }
    }

    fn identifier_columns(&self) -> ColumnIterator {
        match (&self.relation().manifestation, &self.relation_side) {
            (RelationTable(ref m), RelationSide::A) => ColumnIterator::from(vec![m.model_a_column.clone().into()]),
            (RelationTable(ref m), RelationSide::B) => ColumnIterator::from(vec![m.model_b_column.clone().into()]),
            _ => self.model().primary_identifier().as_columns(),
        }
    }

    fn as_table(&self) -> Table<'static> {
        if self.relation().is_many_to_many() {
            self.related_field().relation().as_table()
        } else {
            self.model().as_table()
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
            // In this case we must define our unique indices for the relation
            // table, so MSSQL can convert the `INSERT .. ON CONFLICT IGNORE` into
            // a `MERGE` statement.
            RelationLinkManifestation::RelationTable(ref m) => {
                let db = self.model_a().internal_data_model().db_name.clone();
                let table: Table = (db, m.table.clone()).into();

                table.add_unique_index(vec![Column::from("A"), Column::from("B")])
            }
            RelationLinkManifestation::Inline(ref m) => self
                .internal_data_model()
                .find_model(&m.in_table_of_model_name)
                .unwrap()
                .as_table(),
        }
    }
}
