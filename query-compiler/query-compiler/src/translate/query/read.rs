use std::collections::HashSet;

use crate::{
    expression::{Binding, Expression, JoinExpression},
    translate::TranslateResult,
    TranslateError,
};
use itertools::Itertools;
use query_builder::{QueryArgumentsExt, QueryBuilder, RelationLink};
use query_core::{FilteredQuery, ReadQuery};
use query_structure::{
    ConditionValue, FieldSelection, Filter, PrismaValue, QueryArguments, QueryMode, RelationField, ScalarCondition,
    ScalarFilter, ScalarProjection,
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

        ReadQuery::RelatedRecordsQuery(_) => unreachable!("related records query should not be at the top-level"),

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
        .map(|rrq| -> TranslateResult<JoinExpression> {
            let parent_field_name = rrq.parent_field.name().to_owned();

            let conditions = rrq
                .parent_field
                .left_scalars()
                .into_iter()
                .map(|field| {
                    let placeholder = PrismaValue::placeholder(
                        format!("@parent${}", field.name()),
                        field.type_identifier().to_placeholder_type(),
                    );
                    if parent.r#type().is_list() {
                        ScalarCondition::InTemplate(ConditionValue::value(placeholder))
                    } else {
                        ScalarCondition::Equals(ConditionValue::value(placeholder))
                    }
                })
                .collect();

            let selected_fields = rrq.selected_fields.without_relations().into_virtuals_last();
            let needs_reversed_order = rrq.args.needs_reversed_order();

            let (mut child_query, join_on) = if rrq.parent_field.relation().is_many_to_many() {
                build_read_m2m_query(rrq.parent_field, conditions, rrq.args, &selected_fields, builder)?
            } else {
                build_read_one2m_query(rrq.parent_field, conditions, rrq.args, &selected_fields, builder)?
            };

            if needs_reversed_order {
                child_query = Expression::Reverse(Box::new(child_query));
            }

            if !rrq.nested.is_empty() {
                child_query = add_inmemory_join(child_query, rrq.nested, builder)?;
            };

            Ok(JoinExpression {
                child: child_query,
                on: join_on,
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

fn build_read_m2m_query(
    field: RelationField,
    mut conditions: Vec<ScalarCondition>,
    args: QueryArguments,
    selected_fields: &FieldSelection,
    builder: &dyn QueryBuilder,
) -> TranslateResult<(Expression, Vec<(String, String)>)> {
    let condition = conditions
        .pop()
        .expect("should have at least one condition in m2m relation");
    assert!(
        conditions.is_empty(),
        "should have at most one condition in m2m relation"
    );

    let link = RelationLink::new(field, condition);
    let join_expr = link
        .field()
        .linking_fields()
        .scalars()
        .map(|left| (left.name().to_owned(), link.to_string()))
        .collect_vec();

    let query = builder
        .build_get_related_records(link, args, selected_fields)
        .map_err(TranslateError::QueryBuildFailure)?;

    Ok((Expression::Query(query), join_expr))
}

fn build_read_one2m_query(
    field: RelationField,
    conditions: Vec<ScalarCondition>,
    mut args: QueryArguments,
    selected_fields: &FieldSelection,
    builder: &dyn QueryBuilder,
) -> TranslateResult<(Expression, Vec<(String, String)>)> {
    let join_expr = field
        .linking_fields()
        .scalars()
        .zip(field.related_field().left_scalars())
        .map(|(left, right)| (left.name().to_owned(), right.name().to_owned()))
        .collect_vec();

    // TODO: we ignore chunking for now
    let linking_scalars = field.related_field().left_scalars();

    assert_eq!(
        linking_scalars.len(),
        conditions.len(),
        "linking fields should match conditions"
    );
    for (condition, child_field) in conditions.into_iter().zip(linking_scalars) {
        args.add_filter(Filter::Scalar(ScalarFilter {
            condition,
            projection: ScalarProjection::Single(child_field.clone()),
            mode: QueryMode::Default,
        }));
    }

    let to_one_relation = !field.arity().is_list();
    let args = if to_one_relation { args.with_take(Some(1)) } else { args };
    let query = builder
        .build_get_records(&field.related_model(), args, selected_fields)
        .map_err(TranslateError::QueryBuildFailure)?;

    let mut expr = Expression::Query(query);
    if to_one_relation {
        expr = Expression::Unique(Box::new(expr));
    }
    Ok((expr, join_expr))
}
