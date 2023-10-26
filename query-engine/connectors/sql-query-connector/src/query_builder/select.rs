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

    let select = with_related_queries(select, nested, ctx);
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

fn with_related_queries<'a>(input: Select<'a>, related_queries: Vec<RelatedQuery>, ctx: &Context<'_>) -> Select<'a> {
    related_queries.into_iter().fold(input, |acc, rq| {
        let alias = join_alias_name(&rq.parent_field);
        let is_m2m = rq.parent_field.relation().is_many_to_many();

        let join_columns = rq
            .parent_field
            .join_columns(ctx)
            .map(|c| c.opt_table(is_m2m.then(|| m2m_join_alias_name(&rq.parent_field))));
        let related_alias = join_alias_name(&rq.parent_field.related_field());
        let related_join_columns = ModelProjection::from(rq.parent_field.related_field().linking_fields())
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

        let m2m_join = build_m2m_join(&rq, ctx);

        // LEFT JOIN LATERAL () AS <relation name> ON TRUE
        let join_select = Table::from(build_related_query_select(rq, ctx).and_where(join_cond))
            .alias(alias)
            .on(ConditionTree::single(true.raw()));

        if is_m2m {
            // m2m relations need to left join on the relation table first
            acc.join(m2m_join).left_join_lateral(join_select)
        } else {
            acc.left_join_lateral(join_select)
        }
    })
}

fn build_m2m_join(rq: &RelatedQuery, ctx: &Context<'_>) -> Join<'static> {
    let m2m_table = rq
        .parent_field
        .as_table(ctx)
        .alias(m2m_join_alias_name(&rq.parent_field));

    let left_columns = rq.parent_field.identifier_columns(ctx);
    let right_columns = ModelProjection::from(rq.parent_field.model().primary_identifier()).as_columns(ctx);

    let conditions = left_columns
        .zip(right_columns)
        .fold(None::<ConditionTree>, |acc, (a, b)| match acc {
            Some(acc) => Some(acc.and(a.equals(b))),
            None => Some(a.equals(b).into()),
        })
        .unwrap();

    let m2m_join = m2m_table.on(conditions);

    Join::Left(m2m_join)
}

pub(crate) fn build_related_query_select(related_query: RelatedQuery, ctx: &Context<'_>) -> Select<'static> {
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
        with_related_queries(inner, nested, ctx)
    } else {
        inner
    };

    let inner = Table::from(inner).alias(inner_alias);

    let select = Select::from_table(inner).value(
        coalesce(vec![
            json_array_agg(Column::from(JSON_AGG_IDENT)).into(),
            Expression::from("[]".raw()),
        ])
        .alias(JSON_AGG_IDENT),
    );

    select
}

fn join_alias_name(rf: &RelationField) -> String {
    format!("{}_{}", rf.model().name(), rf.name())
}

fn m2m_join_alias_name(rf: &RelationField) -> String {
    format!("{}_{}_m2m", rf.model().name(), rf.name())
}
