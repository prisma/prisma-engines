use std::borrow::Cow;

use crate::{
    context::Context,
    filter::FilterBuilder,
    model_extensions::{AsColumn, AsColumns, AsTable, RelationFieldExt},
};

use connector_interface::{QueryArguments, RelAggregationSelection, RelatedQuery};
use itertools::Itertools;
use prisma_models::{ModelProjection, RelationField};
use quaint::{prelude::*, visitor::*};

pub const JSON_AGG_IDENT: &str = "data";

pub(crate) fn build(
    args: QueryArguments,
    nested: Vec<RelatedQuery>,
    selection: &ModelProjection,
    _aggr_selections: &[RelAggregationSelection],
    ctx: &Context<'_>,
) -> Select<'static> {
    // SELECT ... FROM Table
    let select = Select::from_table(args.model().as_table(ctx));

    // scalars selection
    let select = selection
        .scalar_fields()
        .fold(select, |acc, sf| acc.column(sf.as_column(ctx)));

    // TODO: check how to select aggregated relations
    let select = nested.iter().fold(select, |acc, read| {
        acc.value(Column::from((join_alias_name(&read.parent_field), JSON_AGG_IDENT)).alias(read.name.to_owned()))
    });

    let select = with_nested_joins(select, nested, ctx);
    let select = with_pagination_and_filters(select, args, ctx);

    let (sql, _) = Postgres::build(select.clone()).unwrap();

    println!("{}", sql);

    select
}

fn with_pagination_and_filters<'a>(select: Select<'a>, args: QueryArguments, ctx: &Context<'_>) -> Select<'a> {
    let (filter, joins) = match args.filter {
        Some(filter) => {
            let (filter, joins) = FilterBuilder::with_top_level_joins().visit_filter(filter, ctx);

            (Some(filter), joins)
        }
        None => (None, None),
    };

    let select = match filter {
        Some(filter) => select.and_where(filter),
        None => select,
    };

    let select = match joins {
        Some(joins) => joins.into_iter().fold(select, |acc, join| acc.join(join.data)),
        None => select,
    };

    let select = match args.take {
        Some(take) => select.limit(take as usize),
        None => select,
    };

    let select = match args.skip {
        Some(skip) => select.offset(skip as usize),
        None => select,
    };

    select
}

pub(crate) fn build_nested(related_query: RelatedQuery, ctx: &Context<'_>) -> Select<'static> {
    let mut build_obj_params = ModelProjection::from(related_query.selected_fields)
        .fields()
        .map(|f| match f {
            prisma_models::Field::Scalar(sf) => {
                (Cow::from(sf.db_name().to_owned()), Expression::from(sf.as_column(ctx)))
            }
            _ => unreachable!(),
        })
        .collect_vec();

    if let Some(nested_queries) = &related_query.nested {
        for nested_query in nested_queries {
            build_obj_params.push((
                Cow::from(nested_query.name.to_owned()),
                Expression::from(Column::from((
                    join_alias_name(&nested_query.parent_field),
                    JSON_AGG_IDENT,
                ))),
            ));
        }
    }

    let inner_alias = join_alias_name(&related_query.parent_field.related_field());

    // SELECT JSON_BUILD_OBJECT()
    let inner = Select::from_table(related_query.parent_field.related_model().as_table(ctx))
        .value(json_build_object(build_obj_params).alias(JSON_AGG_IDENT));

    // SELECT <foreign_keys>
    let inner = ModelProjection::from(related_query.parent_field.related_field().linking_fields())
        .as_columns(ctx)
        .fold(inner, |acc, c| acc.column(c));

    let inner = with_pagination_and_filters(inner, related_query.args, ctx);

    let inner = if let Some(nested) = related_query.nested {
        with_nested_joins(inner, nested, ctx)
    } else {
        inner
    };

    let inner = Table::from(inner).alias(inner_alias);

    let select = Select::from_table(inner).value(json_array_agg(Column::from(JSON_AGG_IDENT)).alias(JSON_AGG_IDENT));

    select
}

fn with_nested_joins<'a>(input: Select<'a>, nested: Vec<RelatedQuery>, ctx: &Context<'_>) -> Select<'a> {
    nested.into_iter().fold(input, |acc, nested| {
        let alias = join_alias_name(&nested.parent_field);

        let join_columns = nested.parent_field.join_columns(ctx);
        let related_alias = join_alias_name(&nested.parent_field.related_field());
        let related_join_columns = ModelProjection::from(nested.parent_field.related_field().linking_fields())
            .as_columns(ctx)
            .map(|c| c.table(related_alias.clone()));
        // WHERE Parent.id = Child.id
        let join_cond = join_columns
            .zip(related_join_columns)
            .fold(None::<ConditionTree>, |acc, (a, b)| match acc {
                Some(acc) => Some(acc.and(a.equals(b))),
                None => Some(a.equals(b).into()),
            })
            .unwrap();

        // LEFT JOIN LATERAL () AS <relation name> ON TRUE
        let join_select = Table::from(build_nested(nested, ctx).and_where(join_cond))
            .alias(alias)
            .on(ConditionTree::single(true.raw()));

        acc.left_join_lateral(join_select)
    })
}

fn join_alias_name(rf: &RelationField) -> String {
    format!("{}_{}", rf.model().name(), rf.name())
}
