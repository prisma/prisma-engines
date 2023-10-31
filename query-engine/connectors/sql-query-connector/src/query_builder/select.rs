use std::borrow::Cow;

use crate::{
    context::Context,
    filter::FilterBuilder,
    model_extensions::{AsColumn, AsColumns, AsTable, RelationFieldExt},
    ordering::OrderByBuilder,
};

use connector_interface::{Filter, QueryArguments, RelAggregationSelection, RelatedQuery};
use itertools::Itertools;
use prisma_models::{ModelProjection, RelationField, ScalarField};
use quaint::prelude::*;

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
    // Adds relation selections to the top-level query
    let select = nested.iter().fold(select, |acc, read| {
        let table_name = match read.parent_field.relation().is_many_to_many() {
            true => m2m_join_alias_name(&read.parent_field),
            false => join_alias_name(&read.parent_field),
        };

        acc.value(Column::from((table_name, JSON_AGG_IDENT)).alias(read.name.to_owned()))
    });

    // Adds joins for relations
    let select = with_related_queries(select, nested, ctx);
    let select = with_ordering(select, &args, None, ctx);
    let select = with_pagination(select, args.take, args.skip);
    let select = with_filters(select, args.filter, ctx);

    select
}

fn with_related_queries<'a>(input: Select<'a>, related_queries: Vec<RelatedQuery>, ctx: &Context<'_>) -> Select<'a> {
    related_queries
        .into_iter()
        .fold(input, |acc, rq| with_related_query(acc, rq, ctx))
}

fn with_related_query<'a>(select: Select<'a>, rq: RelatedQuery, ctx: &Context<'_>) -> Select<'a> {
    if rq.parent_field.relation().is_many_to_many() {
        let m2m_join = build_m2m_join(rq, ctx);

        // m2m relations need to left join on the relation table first
        select.left_join(m2m_join)
    } else {
        let alias = join_alias_name(&rq.parent_field);

        // LEFT JOIN LATERAL () AS <relation name> ON TRUE
        let join_select = Table::from(build_related_query_select(rq, ctx))
            .alias(alias)
            .on(ConditionTree::single(true.raw()))
            .lateral();

        select.left_join(join_select)
    }
}

fn build_related_query_select(rq: RelatedQuery, ctx: &Context<'_>) -> Select<'static> {
    let mut fields_to_select: Vec<ScalarField> = vec![];

    let mut build_obj_params = ModelProjection::from(rq.selected_fields)
        .fields()
        .map(|f| match f {
            prisma_models::Field::Scalar(sf) => {
                (Cow::from(sf.db_name().to_owned()), Expression::from(sf.as_column(ctx)))
            }
            _ => unreachable!(),
        })
        .collect_vec();

    if let Some(nested_queries) = &rq.nested {
        for nested_query in nested_queries {
            let table_name = match nested_query.parent_field.relation().is_many_to_many() {
                true => m2m_join_alias_name(&nested_query.parent_field),
                false => join_alias_name(&nested_query.parent_field),
            };

            build_obj_params.push((
                Cow::from(nested_query.name.to_owned()),
                Expression::from(Column::from((table_name, JSON_AGG_IDENT))),
            ));
        }
    }

    let inner_alias = join_alias_name(&rq.parent_field.related_field());

    // SELECT JSON_BUILD_OBJECT()
    let inner = Select::from_table(rq.parent_field.related_model().as_table(ctx))
        .value(json_build_object(build_obj_params).alias(JSON_AGG_IDENT));

    // SELECT <foreign_keys>
    let inner = ModelProjection::from(rq.parent_field.related_field().linking_fields())
        .as_columns(ctx)
        .fold(inner, |acc, c| acc.column(c));

    let inner = with_join_conditions(inner, &rq.parent_field, ctx);

    let inner = if let Some(nested) = rq.nested {
        with_related_queries(inner, nested, ctx)
    } else {
        inner
    };

    if rq.parent_field.relation().is_many_to_many() {
        // SELECT <orderby columns> ONLY if it's a m2m table as we need to order by outside of the inner select
        let inner = rq
            .args
            .order_by
            .iter()
            .flat_map(|order_by| match order_by {
                prisma_models::OrderBy::Scalar(x) if x.path.is_empty() => vec![x.field.clone()],
                prisma_models::OrderBy::Relevance(x) => x.fields.clone(),
                _ => Vec::new(),
            })
            .fold(inner, |acc, sf| acc.column(sf.as_column(ctx)));

        inner
    } else {
        let inner = with_ordering(inner, &rq.args, None, ctx);
        let inner = with_pagination(inner, rq.args.take, rq.args.skip);
        let inner = with_filters(inner, rq.args.filter, ctx);

        let inner = Table::from(inner).alias(inner_alias.clone());
        let middle = Select::from_table(inner).column(Column::from((inner_alias.clone(), JSON_AGG_IDENT)));
        let outer = Select::from_table(Table::from(middle).alias(format!("{}_1", inner_alias))).value(json_agg());

        outer
    }
}

fn build_m2m_join<'a>(rq: RelatedQuery, ctx: &Context<'_>) -> JoinData<'a> {
    let rf = rq.parent_field.clone();
    let m2m_alias = m2m_join_alias_name(&rf);

    let left_columns = rf.related_field().m2m_columns(ctx);
    let right_columns = ModelProjection::from(rf.model().primary_identifier()).as_columns(ctx);

    let conditions = left_columns
        .into_iter()
        .zip(right_columns)
        .fold(None::<ConditionTree>, |acc, (a, b)| match acc {
            Some(acc) => Some(acc.and(a.equals(b))),
            None => Some(a.equals(b).into()),
        })
        .unwrap();

    let inner = Select::from_table(rf.as_table(ctx))
        .value(Column::from((join_alias_name(&rf), JSON_AGG_IDENT)))
        .and_where(conditions);

    let inner = with_ordering(inner, &rq.args, Some(join_alias_name(&rq.parent_field)), ctx);
    let inner = with_pagination(inner, rq.args.take, rq.args.skip);
    // TODO: avoid clone?
    let inner = with_filters(inner, rq.args.filter.clone(), ctx);

    let join_select = Table::from(build_related_query_select(rq, ctx))
        .alias(join_alias_name(&rf))
        .on(ConditionTree::single(true.raw()))
        .lateral();

    let inner = inner.left_join(join_select);

    let outer = Select::from_table(Table::from(inner).alias(format!("{}_1", m2m_alias))).value(json_agg());

    Table::from(outer)
        .alias(m2m_alias)
        .on(ConditionTree::single(true.raw()))
        .lateral()
}

fn json_agg() -> Function<'static> {
    coalesce(vec![
        json_array_agg(Column::from(JSON_AGG_IDENT)).into(),
        Expression::from("[]".raw()),
    ])
    .alias(JSON_AGG_IDENT)
}

/// Builds the lateral join conditions
fn with_join_conditions<'a>(select: Select<'a>, rf: &RelationField, ctx: &Context<'_>) -> Select<'a> {
    let join_columns = rf.join_columns(ctx);
    // .map(|c| c.opt_table(is_m2m.then(|| m2m_join_alias_name(rf))));
    let related_join_columns = ModelProjection::from(rf.related_field().linking_fields()).as_columns(ctx);

    // WHERE Parent.id = Child.id
    let conditions = join_columns
        .zip(related_join_columns)
        .fold(None::<ConditionTree>, |acc, (a, b)| match acc {
            Some(acc) => Some(acc.and(a.equals(b))),
            None => Some(a.equals(b).into()),
        })
        .unwrap();

    select.and_where(conditions)
}

fn with_ordering<'a>(
    select: Select<'a>,
    args: &QueryArguments,
    parent_alias: Option<String>,
    ctx: &Context<'_>,
) -> Select<'a> {
    let order_by_definitions = OrderByBuilder::default()
        .with_parent_alias(parent_alias)
        .build(args, ctx);

    let select = order_by_definitions
        .iter()
        .flat_map(|j| &j.joins)
        .fold(select, |acc, join| acc.join(join.clone().data));

    order_by_definitions
        .iter()
        .fold(select, |acc, o| acc.order_by(o.order_definition.clone()))
}

fn with_pagination<'a>(select: Select<'a>, take: Option<i64>, skip: Option<i64>) -> Select<'a> {
    let select = match take {
        Some(take) => select.limit(take as usize),
        None => select,
    };

    let select = match skip {
        Some(skip) => select.offset(skip as usize),
        None => select,
    };

    select
}

fn with_filters<'a>(select: Select<'a>, filter: Option<Filter>, ctx: &Context<'_>) -> Select<'a> {
    if let Some(filter) = filter {
        let (filter, joins) = FilterBuilder::with_top_level_joins().visit_filter(filter, ctx);
        let select = select.and_where(filter);

        let select = match joins {
            Some(joins) => joins.into_iter().fold(select, |acc, join| acc.join(join.data)),
            None => select,
        };

        select
    } else {
        select
    }
}

fn join_alias_name(rf: &RelationField) -> String {
    format!("{}_{}", rf.model().name(), rf.name())
}

fn m2m_join_alias_name(rf: &RelationField) -> String {
    format!("{}_{}_m2m", rf.model().name(), rf.name())
}
