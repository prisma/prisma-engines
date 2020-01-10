use crate::{AsColumns, AsTable, InlineRelation, Relation, RelationField, RelationSide, ColumnIterator};
use quaint::ast::{Column, Table};

pub trait RelationExt {
    /// A helper function to decide actions based on the `Relation` type. Inline
    /// relation will return columns for updates, a relation table gives back `None`.
    fn inline_relation_columns(&self) -> Option<ColumnIterator>;

    fn id_columns(&self) -> Option<ColumnIterator>;
    fn columns_for_relation_side(&self, side: RelationSide) -> ColumnIterator;
    fn model_a_columns(&self) -> ColumnIterator;
    fn model_b_columns(&self) -> ColumnIterator;
}

pub trait RelationFieldExt {
    fn opposite_columns(&self, alias: bool) -> ColumnIterator;
    fn relation_columns(&self, alias: bool) -> ColumnIterator;
}

pub trait InlineRelationExt {
    fn referencing_columns(&self, table: Table<'static>) -> ColumnIterator;
}

impl InlineRelationExt for InlineRelation {
    fn referencing_columns(&self, table: Table<'static>) -> ColumnIterator {
        let column = Column::from(self.referencing_column.clone());
        ColumnIterator::from(vec![column.table(table)])
    }
}

impl RelationFieldExt for RelationField {
    fn opposite_columns(&self, alias: bool) -> ColumnIterator {
        let cols = match self.relation_side {
            RelationSide::A => self.relation().model_b_columns(),
            RelationSide::B => self.relation().model_a_columns(),
        };

        if alias && !self.relation_is_inlined_in_child() {
            let inner = cols.into_iter().map(|col| col.table(Relation::TABLE_ALIAS));
            ColumnIterator::new(inner)
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
            let inner = cols.into_iter().map(|col| col.table(Relation::TABLE_ALIAS));
            ColumnIterator::new(inner)
        } else {
            cols
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
        use crate::RelationLinkManifestation::*;

        match self.manifestation {
            Some(RelationTable(ref m)) => {
                let db = self.model_a().internal_data_model().db_name.clone();
                (db, m.table.clone()).into()
            }
            Some(Inline(ref m)) => self
                .internal_data_model()
                .find_model(&m.in_table_of_model_name)
                .unwrap()
                .as_table(),
            None => {
                let db = self.model_a().internal_data_model().db_name.clone();
                (db, format!("_{}", self.name)).into()
            }
        }
    }
}

impl RelationExt for Relation {
    fn id_columns(&self) -> Option<ColumnIterator> {
        use crate::RelationLinkManifestation::*;

        match self.manifestation {
            None => Some(ColumnIterator::from(vec!["id".into()])),
            Some(RelationTable(ref m)) => m.id_column.as_ref().map(|s| {
                let st: String = s.clone();
                ColumnIterator::from(vec![st.into()])
            }),
            _ => None,
        }
    }

    fn columns_for_relation_side(&self, side: RelationSide) -> ColumnIterator {
        match side {
            RelationSide::A => self.model_a_columns(),
            RelationSide::B => self.model_b_columns(),
        }
    }

    fn inline_relation_columns(&self) -> Option<ColumnIterator> {
        if let Some(mani) = self.inline_manifestation() {
            Some(ColumnIterator::from(vec![
                Column::from(mani.referencing_column.clone()).table(self.as_table())
            ]))
        } else {
            None
        }
    }

    #[allow(clippy::if_same_then_else)]
    fn model_a_columns(&self) -> ColumnIterator {
        use crate::RelationLinkManifestation::*;

        match self.manifestation {
            Some(RelationTable(ref m)) =>
                ColumnIterator::from(vec![m.model_a_column.clone().into()]),
            Some(Inline(ref m)) => {
                let model_a = self.model_a();
                let model_b = self.model_b();

                if self.is_self_relation() && self.field_a().is_hidden {
                    ColumnIterator::new(model_a.identifier().as_columns())
                } else if self.is_self_relation() && self.field_b().is_hidden {
                    ColumnIterator::new(model_b.identifier().as_columns())
                } else if self.is_self_relation() {
                    m.referencing_columns(self.as_table())
                } else if m.in_table_of_model_name == model_a.name && !self.is_self_relation() {
                    ColumnIterator::new(model_a.identifier().as_columns())
                } else {
                    m.referencing_columns(self.as_table())
                }
            }
            None => ColumnIterator::from(vec![Relation::MODEL_A_DEFAULT_COLUMN.into()]),
        }
    }

    #[allow(clippy::if_same_then_else)]
    fn model_b_columns(&self) -> ColumnIterator {
        use crate::RelationLinkManifestation::*;

        match self.manifestation {
            Some(RelationTable(ref m)) =>
                ColumnIterator::from(vec![m.model_b_column.clone().into()]),
            Some(Inline(ref m)) => {
                let model_b = self.model_b();

                if self.is_self_relation() && (self.field_a().is_hidden || self.field_b().is_hidden) {
                    m.referencing_columns(self.as_table())
                } else if self.is_self_relation() {
                    ColumnIterator::new(model_b.identifier().as_columns())
                } else if m.in_table_of_model_name == model_b.name && !self.is_self_relation() {
                    ColumnIterator::new(model_b.identifier().as_columns())
                } else {
                    m.referencing_columns(self.as_table())
                }
            }
            None => ColumnIterator::from(vec![Relation::MODEL_B_DEFAULT_COLUMN.into()]),
        }
    }
}
