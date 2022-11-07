use crate::join_utils::{AliasedJoin, JoinType};
use crate::model_extensions::AsColumn;
use crate::query_builder::QueryBuilderContext;
use connector_interface::NestedRead;
use prisma_models::RelationFieldRef;
use quaint::ast::*;
use quaint::prelude::{Column, Expression};

pub fn build_columns(
    ctx: &mut QueryBuilderContext,
    nested_reads: &[NestedRead],
    parents: Vec<RelationFieldRef>,
    depth: usize,
) -> Vec<Expression<'static>> {
    let mut columns: Vec<Expression<'static>> = vec![];

    for read in nested_reads.iter() {
        let mut parents = parents.clone();
        parents.push(read.parent_field.clone());

        let join = ctx.joins().last(&parents, JoinType::Normal).unwrap();

        for (i, selection) in read.selected_fields.selections().enumerate() {
            let col: Expression =
                Column::from((join.alias.to_owned(), selection.as_scalar().unwrap().as_column())).into();

            columns.push(col.alias(read.db_alias(i, depth)));
        }

        let nested = build_columns(ctx, &read.nested, parents.clone(), depth + 1);

        columns.extend(nested);
    }

    columns
}
