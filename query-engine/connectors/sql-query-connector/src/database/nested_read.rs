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
    nested_reads: &[NestedRead],
    previous_join: Option<&AliasedJoin>,
) -> NestedReadJoins {
    let mut output: Vec<AliasedJoin> = vec![];
    let mut columns: Vec<Expression<'static>> = vec![];

    for read in nested_reads.iter() {
        let joins = ctx
            .join_builder()
            .compute_join(&read.parent_field, read.args.filter.as_ref(), previous_join);

        let join = joins.last().unwrap();

        for (i, selection) in read.selected_fields.selections().enumerate() {
            let col: Expression =
                Column::from((join.alias.to_owned(), selection.as_scalar().unwrap().as_column())).into();

            columns.push(col.alias(read.db_alias(i)));
        }

        let nested = build_joins(ctx, &read.nested, Some(&join));

        output.extend(joins);
        output.extend(nested.joins);

        columns.extend(nested.columns);
    }

    NestedReadJoins { joins: output, columns }
}
