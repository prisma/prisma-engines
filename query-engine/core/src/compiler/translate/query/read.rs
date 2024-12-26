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
    let all_linking_fields = query
        .nested_related_records_queries()
        .flat_map(|rrq| rrq.parent_field.linking_fields())
        .collect::<HashSet<_>>();

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

            if rq.nested.is_empty() {
                return Ok(expr);
            }

            Expression::Let {
                bindings: vec![Binding {
                    name: "@parent".into(),
                    expr,
                }],
                expr: Box::new(Expression::Let {
                    bindings: all_linking_fields
                        .into_iter()
                        .map(|sf| Binding {
                            name: format!("@parent.{}", sf.prisma_name().into_owned()),
                            expr: Expression::MapField {
                                field: sf.prisma_name().into_owned(),
                                records: Box::new(Expression::Get { name: "@parent".into() }),
                            },
                        })
                        .collect(),
                    expr: Box::new(Expression::Join {
                        parent: Box::new(Expression::Get { name: "@parent".into() }),
                        children: rq
                            .nested
                            .into_iter()
                            .filter_map(|nested| match nested {
                                ReadQuery::RelatedRecordsQuery(rrq) => Some(rrq),
                                _ => None,
                            })
                            .map(|rrq| -> TranslateResult<JoinExpression> {
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
                                })
                            })
                            .try_collect()?,
                    }),
                }),
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

            if needs_reversed_order {
                Expression::Reverse(Box::new(expr))
            } else {
                expr
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

    if needs_reversed_order {
        Ok(Expression::Reverse(Box::new(expr)))
    } else {
        Ok(expr)
    }
}

fn collect_referenced_fields(nested_queries: &[ReadQuery]) -> HashSet<ScalarField> {
    nested_queries
        .iter()
        .filter_map(|rq| match rq {
            ReadQuery::RelatedRecordsQuery(rrq) => Some(rrq),
            _ => None,
        })
        .flat_map(|rrq| rrq.parent_field.referenced_fields())
        .collect()
}
