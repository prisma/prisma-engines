use crate::{AsColumn, AsTable, InlineRelation, Relation, RelationField, RelationLinkManifestation, RelationSide};
use quaint::ast::{Column, Table};

pub trait RelationExt {
    fn id_column(&self) -> Option<Column<'static>>;
    fn column_for_relation_side(&self, side: RelationSide) -> Column<'static>;
    fn model_a_column(&self) -> Column<'static>;
    fn model_b_column(&self) -> Column<'static>;
}

pub trait RelationFieldExt {
    fn opposite_column(&self, alias: bool) -> Column<'static>;
    fn relation_column(&self, alias: bool) -> Column<'static>;
}

pub trait InlineRelationExt {
    fn referencing_column(&self, table: Table<'static>) -> Column<'static>;
}

impl InlineRelationExt for InlineRelation {
    fn referencing_column(&self, table: Table<'static>) -> Column<'static> {
        let column = Column::from(self.referencing_column.clone());
        column.table(table)
    }
}

impl RelationFieldExt for RelationField {
    fn opposite_column(&self, alias: bool) -> Column<'static> {
        let col = match self.relation_side {
            RelationSide::A => self.relation().model_b_column(),
            RelationSide::B => self.relation().model_a_column(),
        };

        if alias && !self.relation_is_inlined_in_child() {
            col.table(Relation::TABLE_ALIAS)
        } else {
            col
        }
    }

    fn relation_column(&self, alias: bool) -> Column<'static> {
        let col = match self.relation_side {
            RelationSide::A => self.relation().model_a_column(),
            RelationSide::B => self.relation().model_b_column(),
        };

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
            Some(RelationLinkManifestation::RelationTable(ref m)) => {
                let db = self.model_a().internal_data_model().db_name.clone();
                (db, m.table.clone()).into()
            }
            Some(RelationLinkManifestation::Inline(ref m)) => self
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
    fn id_column(&self) -> Option<Column<'static>> {
        match self.manifestation {
            None => Some("id".into()),
            Some(RelationLinkManifestation::RelationTable(ref m)) => m.id_column.as_ref().map(|s| {
                let st: String = s.clone();
                st.into()
            }),
            _ => None,
        }
    }

    fn column_for_relation_side(&self, side: RelationSide) -> Column<'static> {
        match side {
            RelationSide::A => self.model_a_column(),
            RelationSide::B => self.model_b_column(),
        }
    }

    #[allow(clippy::if_same_then_else)]
    fn model_a_column(&self) -> Column<'static> {
        match self.manifestation {
            Some(RelationLinkManifestation::RelationTable(ref m)) => m.model_a_column.clone().into(),
            Some(RelationLinkManifestation::Inline(ref m)) => {
                let model_a = self.model_a();
                let model_b = self.model_b();

                if self.is_self_relation() && self.field_a().is_hidden {
                    model_a.fields().id().as_column()
                } else if self.is_self_relation() && self.field_b().is_hidden {
                    model_b.fields().id().as_column()
                } else if self.is_self_relation() {
                    m.referencing_column(self.as_table())
                } else if m.in_table_of_model_name == model_a.name && !self.is_self_relation() {
                    model_a.fields().id().as_column()
                } else {
                    m.referencing_column(self.as_table())
                }
            }
            None => Relation::MODEL_A_DEFAULT_COLUMN.into(),
        }
    }

    #[allow(clippy::if_same_then_else)]
    fn model_b_column(&self) -> Column<'static> {
        match self.manifestation {
            Some(RelationLinkManifestation::RelationTable(ref m)) => m.model_b_column.clone().into(),
            Some(RelationLinkManifestation::Inline(ref m)) => {
                let model_b = self.model_b();

                if self.is_self_relation() && (self.field_a().is_hidden || self.field_b().is_hidden) {
                    m.referencing_column(self.as_table())
                } else if self.is_self_relation() {
                    model_b.fields().id().as_column()
                } else if m.in_table_of_model_name == model_b.name && !self.is_self_relation() {
                    model_b.fields().id().as_column()
                } else {
                    m.referencing_column(self.as_table())
                }
            }
            None => Relation::MODEL_B_DEFAULT_COLUMN.into(),
        }
    }
}
