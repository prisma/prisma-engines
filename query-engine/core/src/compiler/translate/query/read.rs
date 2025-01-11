use std::collections::HashSet;

use itertools::Itertools;
use query_structure::{
    ConditionValue, Filter, ModelProjection, PlaceholderType, PrismaValue, QueryMode, RelationField, ScalarCondition,
    ScalarField, ScalarFilter, ScalarProjection,
};
use sql_query_connector::{
    context::Context, model_extensions::AsColumns, query_arguments_ext::QueryArgumentsExt, query_builder,
};

use crate::{
    compiler::{
        expression::{Binding, Expression, JoinExpression},
        translate::TranslateResult,
    },
    FilteredQuery, ReadQuery, RelatedRecordsQuery,
};

use super::build_db_query;

pub(crate) fn translate_read_query(query: ReadQuery, ctx: &Context<'_>) -> TranslateResult<Expression> {
    Ok(match query {
        ReadQuery::RecordQuery(rq) => {
            let selected_fields = rq.selected_fields.without_relations().into_virtuals_last();

            let query = query_builder::read::get_records(
                &rq.model,
                ModelProjection::from(&selected_fields)
                    .as_columns(ctx)
                    .mark_all_selected(),
                selected_fields.virtuals(),
                rq.filter.expect("ReadOne query should always have filter set"),
                ctx,
            )
            .limit(1);

            let expr = Expression::Query(build_db_query(query)?);
            let expr = Expression::Unique(Box::new(expr));

            if rq.nested.is_empty() {
                expr
            } else {
                add_inmemory_join(expr, rq.nested, ctx)?
            }
        }

        ReadQuery::ManyRecordsQuery(mrq) => {
            let selected_fields = mrq.selected_fields.without_relations().into_virtuals_last();
            let needs_reversed_order = mrq.args.needs_reversed_order();

            // TODO: we ignore chunking for now
            let query = query_builder::read::get_records(
                &mrq.model,
                ModelProjection::from(&selected_fields)
                    .as_columns(ctx)
                    .mark_all_selected(),
                selected_fields.virtuals(),
                mrq.args,
                ctx,
            );

            let expr = Expression::Query(build_db_query(query)?);

            let expr = if needs_reversed_order {
                Expression::Reverse(Box::new(expr))
            } else {
                expr
            };

            if mrq.nested.is_empty() {
                expr
            } else {
                add_inmemory_join(expr, mrq.nested, ctx)?
            }
        }

        ReadQuery::RelatedRecordsQuery(rrq) => {
            if rrq.parent_field.relation().is_many_to_many() {
                build_read_m2m_query(rrq, ctx)?
            } else {
                build_read_one2m_query(rrq, ctx)?
            }
        }

        _ => todo!(),
    })
}

fn add_inmemory_join(parent: Expression, nested: Vec<ReadQuery>, ctx: &Context<'_>) -> TranslateResult<Expression> {
    let all_linking_fields = nested
        .iter()
        .flat_map(|nested| match nested {
            ReadQuery::RelatedRecordsQuery(rrq) => rrq.parent_field.linking_fields(),
            _ => unreachable!(),
        })
        .collect::<HashSet<_>>();

    let linking_fields_bindings = all_linking_fields
        .into_iter()
        .map(|sf| Binding {
            name: format!("@parent${}", sf.prisma_name().into_owned()),
            expr: Expression::MapField {
                field: sf.prisma_name().into_owned(),
                records: Box::new(Expression::Get { name: "@parent".into() }),
            },
        })
        .collect();

    let join_expressions = nested
        .into_iter()
        .filter_map(|nested| match nested {
            ReadQuery::RelatedRecordsQuery(rrq) => Some(rrq),
            _ => None,
        })
        .map(|rrq| -> TranslateResult<JoinExpression> {
            let parent_field_name = rrq.parent_field.name().to_owned();
            let parent_fields = rrq.parent_field.linking_fields();
            let child_fields = rrq.parent_field.related_field().linking_fields();

            let join_expr = parent_fields
                .scalars()
                .zip(child_fields.scalars())
                .map(|(left, right)| (left.name().to_owned(), right.name().to_owned()))
                .collect_vec();

            // nested.add_filter(Filter::Scalar(ScalarFilter {
            //     mode: QueryMode::Default,
            //     condition: ScalarCondition::Equals(ConditionValue::value(PrismaValue::placeholder(
            //         "parent_id".into(),
            //         PlaceholderType::String,
            //     ))),
            //     projection: ScalarProjection::Compound(referenced_fields),
            // }));
            let child_query = translate_read_query(ReadQuery::RelatedRecordsQuery(rrq), ctx)?;

            Ok(JoinExpression {
                child: child_query,
                on: join_expr,
                parent_field: parent_field_name,
            })
        })
        .try_collect()?;

    Ok(Expression::Let {
        bindings: vec![Binding {
            name: "@parent".into(),
            expr: parent,
        }],
        expr: Box::new(Expression::Let {
            bindings: linking_fields_bindings,
            expr: Box::new(Expression::Join {
                parent: Box::new(Expression::Get { name: "@parent".into() }),
                children: join_expressions,
            }),
        }),
    })
}

fn build_read_m2m_query(_query: RelatedRecordsQuery, _ctx: &Context<'_>) -> TranslateResult<Expression> {
    todo!()
}

fn build_read_one2m_query(rrq: RelatedRecordsQuery, ctx: &Context<'_>) -> TranslateResult<Expression> {
    let selected_fields = rrq.selected_fields.without_relations().into_virtuals_last();
    let needs_reversed_order = rrq.args.needs_reversed_order();

    // TODO: we ignore chunking for now
    let query = query_builder::read::get_records(
        &rrq.parent_field.related_model(),
        ModelProjection::from(&selected_fields)
            .as_columns(ctx)
            .mark_all_selected(),
        selected_fields.virtuals(),
        rrq.args,
        ctx,
    );

    let expr = Expression::Query(build_db_query(query)?);

    let expr = if needs_reversed_order {
        Expression::Reverse(Box::new(expr))
    } else {
        expr
    };

    if rrq.nested.is_empty() {
        Ok(expr)
    } else {
        add_inmemory_join(expr, rrq.nested, ctx)
    }
}
