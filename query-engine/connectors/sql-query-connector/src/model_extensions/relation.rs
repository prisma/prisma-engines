use crate::{
    model_extensions::{AsColumns, AsTable, ColumnIterator},
    Context,
};
use quaint::{ast::Table, prelude::Column};
use query_structure::{walkers, ModelProjection, Relation, RelationField};

pub(crate) trait RelationFieldExt {
    fn m2m_columns(&self, ctx: &Context<'_>) -> Vec<Column<'static>>;
    fn join_columns(&self, ctx: &Context<'_>) -> ColumnIterator;
    fn identifier_columns(&self, ctx: &Context<'_>) -> ColumnIterator;
    fn as_table(&self, ctx: &Context<'_>) -> Table<'static>;
}

impl RelationFieldExt for RelationField {
    fn m2m_columns(&self, ctx: &Context<'_>) -> Vec<Column<'static>> {
        let is_side_a = self.walker().relation().relation_fields().next().map(|rf| rf.id) == Some(self.id);
        let prefix = if is_side_a { "B" } else { "A" };
        vec![Column::from(prefix).table(self.as_table(ctx))]
    }

    fn join_columns(&self, ctx: &Context<'_>) -> ColumnIterator {
        let relation = self.walker().relation();
        match relation.refine() {
            walkers::RefinedRelationWalker::ImplicitManyToMany(m) => {
                let is_side_a = relation.relation_fields().next().map(|rf| rf.id) == Some(self.id);
                let column_name = if is_side_a {
                    m.column_b_name()
                } else {
                    m.column_a_name()
                };
                ColumnIterator::from(vec![column_name.into()])
            }
            _ => ModelProjection::from(self.linking_fields()).as_columns(ctx),
        }
    }

    fn identifier_columns(&self, ctx: &Context<'_>) -> ColumnIterator {
        let relation = self.walker().relation();
        match relation.refine() {
            walkers::RefinedRelationWalker::ImplicitManyToMany(m) => {
                let is_side_a = relation.relation_fields().next().map(|rf| rf.id) == Some(self.id);
                let column_name = if is_side_a {
                    m.column_a_name()
                } else {
                    m.column_b_name()
                };
                ColumnIterator::from(vec![column_name.into()])
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
                let model_a = m.model_a();
                let prefix = model_a.schema_name().unwrap_or_else(|| ctx.schema_name()).to_owned();
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
