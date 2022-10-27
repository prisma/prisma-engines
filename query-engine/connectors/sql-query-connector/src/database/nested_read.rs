use crate::join_utils::AliasedJoin;
use crate::model_extensions::{AsColumn, AsTable};
use crate::query_builder::QueryBuilderContext;
use connector_interface::NestedRead;
use quaint::ast::*;
use quaint::prelude::{Column, Expression};

#[derive(Debug)]
pub struct NestedReadJoins {
    pub joins: Vec<AliasedJoin>,
    pub columns: Vec<Expression<'static>>,
}

pub fn build_joins(
    ctx: &mut QueryBuilderContext,
    nested: &[NestedRead],
    previous_join: Option<&AliasedJoin>,
) -> NestedReadJoins {
    let mut joins: Vec<AliasedJoin> = vec![];
    let mut columns: Vec<Expression<'static>> = vec![];

    for (a, read) in nested.iter().enumerate() {
        let join = if read.parent_field.relation().is_one_to_many() {
            ctx.join_builder().compute_one2m_join(&read.parent_field, previous_join)
        } else if read.parent_field.relation().is_one_to_one() {
            ctx.join_builder().compute_one2m_join(&read.parent_field, previous_join)
        } else {
            todo!("m2m not supported yet")
        };

        for (i, selection) in read.selected_fields.selections().enumerate() {
            let col: Expression =
                Column::from((join.alias.to_owned(), selection.as_scalar().unwrap().as_column())).into();

            columns.push(col.alias(read.db_alias(i)));
        }

        let nested = build_joins(ctx, &read.nested, Some(&join));

        joins.push(join);
        joins.extend(nested.joins);

        columns.extend(nested.columns);
    }

    NestedReadJoins { joins, columns }
}
