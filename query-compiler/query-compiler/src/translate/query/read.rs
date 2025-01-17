use std::collections::HashSet;

use crate::{
    expression::{Binding, Expression, JoinExpression},
    translate::TranslateResult,
    TranslateError,
};
use itertools::Itertools;
use query_builder::{QueryArgumentsExt, QueryBuilder};
use query_core::{FilteredQuery, ReadQuery, RelatedRecordsQuery};
use query_structure::{
    ConditionValue, Filter, PrismaValue, QueryArguments, QueryMode, ScalarCondition, ScalarFilter, ScalarProjection,
};

pub(crate) fn translate_read_query(query: ReadQuery, builder: &dyn QueryBuilder) -> TranslateResult<Expression> {
    Ok(match query {
        ReadQuery::RecordQuery(rq) => {
            let selected_fields = rq.selected_fields.without_relations().into_virtuals_last();

            let args = QueryArguments::from((
                rq.model.clone(),
                rq.filter.expect("ReadOne query should always have filter set"),
            ))
            .with_take(Some(1));
            let query = builder
                .build_get_records(&rq.model, args, &selected_fields)
                .map_err(TranslateError::QueryBuildFailure)?;

            let expr = Expression::Query(query);
            let expr = Expression::Unique(Box::new(expr));

            if rq.nested.is_empty() {
                expr
            } else {
                add_inmemory_join(expr, rq.nested, builder)?
            }
        }

        ReadQuery::ManyRecordsQuery(mrq) => {
            let selected_fields = mrq.selected_fields.without_relations().into_virtuals_last();
            let needs_reversed_order = mrq.args.needs_reversed_order();

            // TODO: we ignore chunking for now
            let query = builder
                .build_get_records(&mrq.model, mrq.args, &selected_fields)
                .map_err(TranslateError::QueryBuildFailure)?;

            let expr = Expression::Query(query);

            let expr = if needs_reversed_order {
                Expression::Reverse(Box::new(expr))
            } else {
                expr
            };

            if mrq.nested.is_empty() {
                expr
            } else {
                add_inmemory_join(expr, mrq.nested, builder)?
            }
        }

        ReadQuery::RelatedRecordsQuery(rrq) => {
            if rrq.parent_field.relation().is_many_to_many() {
                build_read_m2m_query(rrq, builder)?
            } else {
                build_read_one2m_query(rrq, builder)?
            }
        }

        _ => todo!(),
    })
}

fn add_inmemory_join(
    parent: Expression,
    nested: Vec<ReadQuery>,
    builder: &dyn QueryBuilder,
) -> TranslateResult<Expression> {
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
        .map(|mut rrq| -> TranslateResult<JoinExpression> {
            let parent_field_name = rrq.parent_field.name().to_owned();
            let parent_fields = rrq.parent_field.linking_fields();
            let child_fields = rrq.parent_field.related_field().linking_fields();

            let join_expr = parent_fields
                .scalars()
                .zip(child_fields.scalars())
                .map(|(left, right)| (left.name().to_owned(), right.name().to_owned()))
                .collect_vec();

            for (parent_field, child_field) in parent_fields.scalars().zip(child_fields.scalars()) {
                let placeholder = PrismaValue::placeholder(
                    format!("@parent${}", parent_field.name()),
                    parent_field.type_identifier().to_placeholder_type(),
                );

                let condition = if parent.r#type().is_list() {
                    ScalarCondition::InTemplate(ConditionValue::value(placeholder))
                } else {
                    ScalarCondition::Equals(ConditionValue::value(placeholder))
                };

                rrq.add_filter(Filter::Scalar(ScalarFilter {
                    condition,
                    projection: ScalarProjection::Single(child_field.clone()),
                    mode: QueryMode::Default,
                }));
            }

            let child_query = translate_read_query(ReadQuery::RelatedRecordsQuery(rrq), builder)?;

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

fn build_read_m2m_query(_query: RelatedRecordsQuery, _builder: &dyn QueryBuilder) -> TranslateResult<Expression> {
    todo!()
}

fn build_read_one2m_query(rrq: RelatedRecordsQuery, builder: &dyn QueryBuilder) -> TranslateResult<Expression> {
    let selected_fields = rrq.selected_fields.without_relations().into_virtuals_last();
    let needs_reversed_order = rrq.args.needs_reversed_order();
    let to_one_relation = !rrq.parent_field.arity().is_list();

    // TODO: we ignore chunking for now

    let args = if to_one_relation {
        rrq.args.with_take(Some(1))
    } else {
        rrq.args
    };
    let query = builder
        .build_get_records(&rrq.parent_field.related_model(), args, &selected_fields)
        .map_err(TranslateError::QueryBuildFailure)?;

    let mut expr = Expression::Query(query);

    if to_one_relation {
        expr = Expression::Unique(Box::new(expr));
    }

    if needs_reversed_order {
        expr = Expression::Reverse(Box::new(expr));
    }

    if rrq.nested.is_empty() {
        Ok(expr)
    } else {
        add_inmemory_join(expr, rrq.nested, builder)
    }
}
