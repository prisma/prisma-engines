use crate::{
    model_extensions::{AsColumns, AsTable, ColumnIterator},
    Context,
};
use prisma_models::{walkers, ModelProjection, Relation, RelationField, RelationSide};
use quaint::{ast::Table, prelude::Column};

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
                .map(|to_field| format!("{prefix}_{to_field}"))
                .map(|name| Column::from(name).table(self.as_table(ctx)))
                .collect()
        } else {
            vec![Column::from(prefix).table(self.as_table(ctx))]
        }
    }

    fn join_columns(&self, ctx: &Context<'_>) -> ColumnIterator {
        match (&self.relation().walker().refine(), &self.relation_side) {
            (walkers::RefinedRelationWalker::ImplicitManyToMany(ref m), RelationSide::A) => {
                ColumnIterator::from(vec![m.column_b_name().into()])
            }
            (walkers::RefinedRelationWalker::ImplicitManyToMany(ref m), RelationSide::B) => {
                ColumnIterator::from(vec![m.column_a_name().into()])
            }
            _ => ModelProjection::from(self.linking_fields()).as_columns(ctx),
        }
    }

    fn identifier_columns(&self, ctx: &Context<'_>) -> ColumnIterator {
        match (&self.relation().walker().refine(), &self.relation_side) {
            (walkers::RefinedRelationWalker::ImplicitManyToMany(ref m), RelationSide::A) => {
                ColumnIterator::from(vec![m.column_a_name().into()])
            }
            (walkers::RefinedRelationWalker::ImplicitManyToMany(ref m), RelationSide::B) => {
                ColumnIterator::from(vec![m.column_b_name().into()])
            }
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
        match self.walker().refine() {
            // In this case we must define our unique indices for the relation
            // table, so MSSQL can convert the `INSERT .. ON CONFLICT IGNORE` into
            // a `MERGE` statement.
            walkers::RefinedRelationWalker::ImplicitManyToMany(ref m) => {
                let model_a = self.model_a();
                let prefix = model_a.schema_name().unwrap_or_else(|| ctx.schema_name().to_owned());
                let table: Table = (prefix, m.table_name().to_string()).into();

                table.add_unique_index(vec![Column::from("A"), Column::from("B")])
            }
            walkers::RefinedRelationWalker::Inline(ref m) => {
                self.dm.find_model_by_id(m.referencing_model().id).as_table(ctx)
            }
            walkers::RefinedRelationWalker::TwoWayEmbeddedManyToMany(_) => {
                unreachable!("TwoWayEmbeddedManyToMany relation in sql-query-connector")
            }
        }
    }
}
