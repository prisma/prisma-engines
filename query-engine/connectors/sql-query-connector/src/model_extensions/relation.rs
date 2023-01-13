use crate::{
    model_extensions::{AsColumns, AsTable, ColumnIterator},
    Context,
};
use prisma_models::{ModelProjection, Relation, RelationField, RelationLinkManifestation, RelationSide};
use quaint::{ast::Table, prelude::Column};
use RelationLinkManifestation::*;

pub(crate) trait RelationFieldExt {
    fn m2m_columns(&self, ctx: &Context<'_>) -> Vec<Column<'static>>;
    fn join_columns(&self, ctx: &Context<'_>) -> ColumnIterator;
    fn identifier_columns(&self, ctx: &Context<'_>) -> ColumnIterator;
    fn as_table(&self, ctx: &Context<'_>) -> Table<'static>;
}

impl RelationFieldExt for RelationField {
    fn m2m_columns(&self, ctx: &Context<'_>) -> Vec<Column<'static>> {
        let references = &self.relation_info().references;
        let prefix = if self.relation_side.is_a() { "B" } else { "A" };

        if references.len() > 1 {
            references
                .iter()
                .map(|to_field| format!("{}_{}", prefix, to_field))
                .map(|name| Column::from(name).table(self.as_table(ctx)))
                .collect()
        } else {
            vec![Column::from(prefix).table(self.as_table(ctx))]
        }
    }

    fn join_columns(&self, ctx: &Context<'_>) -> ColumnIterator {
        match (&self.relation().manifestation(), &self.relation_side) {
            (RelationTable(ref m), RelationSide::A) => ColumnIterator::from(vec![m.model_b_column.clone().into()]),
            (RelationTable(ref m), RelationSide::B) => ColumnIterator::from(vec![m.model_a_column.clone().into()]),
            _ => ModelProjection::from(self.linking_fields()).as_columns(ctx),
        }
    }

    fn identifier_columns(&self, ctx: &Context<'_>) -> ColumnIterator {
        match (&self.relation().manifestation(), &self.relation_side) {
            (RelationTable(ref m), RelationSide::A) => ColumnIterator::from(vec![m.model_a_column.clone().into()]),
            (RelationTable(ref m), RelationSide::B) => ColumnIterator::from(vec![m.model_b_column.clone().into()]),
            _ => ModelProjection::from(self.model().primary_identifier()).as_columns(ctx),
        }
    }

    fn as_table(&self, ctx: &Context<'_>) -> Table<'static> {
        if self.relation().is_many_to_many() {
            self.related_field().relation().as_table(ctx)
        } else {
            self.model().as_table(ctx)
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
    fn as_table(&self, ctx: &Context<'_>) -> Table<'static> {
        match self.manifestation() {
            // In this case we must define our unique indices for the relation
            // table, so MSSQL can convert the `INSERT .. ON CONFLICT IGNORE` into
            // a `MERGE` statement.
            RelationLinkManifestation::RelationTable(ref m) => {
                let model_a = self.model_a();
                let prefix = model_a.schema_name().unwrap_or_else(|| ctx.schema_name().to_owned());
                let table: Table = (prefix, m.table.clone()).into();

                table.add_unique_index(vec![Column::from("A"), Column::from("B")])
            }
            RelationLinkManifestation::Inline(ref m) => self
                .internal_data_model()
                .find_model(&m.in_table_of_model_name)
                .unwrap()
                .as_table(ctx),
        }
    }
}
