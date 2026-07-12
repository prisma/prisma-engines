use crate::{
    TranslateError, binding,
    expression::{Binding, Expression, JoinExpression},
    translate::TranslateResult,
};
use itertools::Itertools;
use query_builder::{ConditionalLink, QueryBuilder, RelationLinkage};
use query_core::{
    AggregateRecordsQuery, DataExpectation, DataOperation, MissingRecord, QueryGraphBuilderError, QueryOption,
    QueryOptions, ReadQuery, RelatedRecordsQuery,
};
use query_structure::{
    ConditionValue, FieldSelection, Filter, Model, Placeholder, PrismaValue, QueryArguments, QueryMode, RelationField,
    RelationLoadStrategy, ScalarCondition, ScalarField, ScalarFilter, ScalarProjection, Take,
};
use std::slice;

mod in_memory_processing;

pub(crate) fn translate_read_query(query: ReadQuery, builder: &dyn QueryBuilder) -> TranslateResult<Expression> {
    Ok(match query {
        ReadQuery::RecordQuery(mut rq) => {
            let selected_fields = match rq.relation_load_strategy {
                RelationLoadStrategy::Join => rq.selected_fields.into_virtuals_last(),
                RelationLoadStrategy::Query => rq.selected_fields.without_relations().into_virtuals_last(),
            };

            let mut args = QueryArguments::from((
                rq.model.clone(),
                rq.filter.expect("ReadOne query should always have filter set"),
            ))
            .with_take(Take::One);

            let in_memory_ops =
                in_memory_processing::extract_in_memory_ops(&mut args, rq.relation_load_strategy, &mut rq.nested);

            let expr = build_get_records(builder, &rq.model, args, &selected_fields, rq.relation_load_strategy)?;
            let expr = in_memory_ops.into_expression(expr);
            let expr = convert_options_to_validation(expr, rq.options);
            let expr = Expression::Unique(Box::new(expr));

            match rq.relation_load_strategy {
                RelationLoadStrategy::Query if !rq.nested.is_empty() => add_inmemory_join(expr, rq.nested, builder)?,
                _ => expr,
            }
        }

        ReadQuery::ManyRecordsQuery(mut mrq) => {
            // Skip the query entirely if the take is 0.
            if mrq.args.take == Take::Some(0) {
                return Ok(Expression::Concat(vec![]));
            }

            let selected_fields = match mrq.relation_load_strategy {
                RelationLoadStrategy::Join => mrq.selected_fields.into_virtuals_last(),
                RelationLoadStrategy::Query => mrq.selected_fields.without_relations().into_virtuals_last(),
            };

            let take = mrq.args.take;

            let in_memory_ops =
                in_memory_processing::extract_in_memory_ops(&mut mrq.args, mrq.relation_load_strategy, &mut mrq.nested);

            let expr = build_get_records(
                builder,
                &mrq.model,
                mrq.args,
                &selected_fields,
                mrq.relation_load_strategy,
            )?;

            let expr = in_memory_ops.into_expression(expr);

            let mut expr = convert_options_to_validation(expr, mrq.options);

            if mrq.relation_load_strategy == RelationLoadStrategy::Query && !mrq.nested.is_empty() {
                expr = add_inmemory_join(expr, mrq.nested, builder)?;
            }

            match take {
                Take::One | Take::NegativeOne => Expression::Unique(Box::new(expr)),
                _ => expr,
            }
        }

        ReadQuery::RelatedRecordsQuery(rrq) => {
            let (expr, join) = build_read_related_records(rrq, vec![], false, builder)?;
            if join.is_relation_unique {
                Expression::Unique(Box::new(expr))
            } else {
                expr
            }
        }

        ReadQuery::AggregateRecordsQuery(AggregateRecordsQuery {
            name: _,
            alias: _,
            // TODO: we're ignoring selection order
            selection_order: _,
            model,
            args,
            selectors,
            group_by,
            having,
        }) => {
            let has_group_by = !group_by.is_empty();
            let query = builder
                .build_aggregate(&model, args, &selectors, group_by, having)
                .map_err(TranslateError::QueryBuildFailure)?;
            let expr = Expression::Query(query);
            if has_group_by {
                expr
            } else {
                Expression::Unique(expr.into())
            }
        }
    })
}

pub(super) fn add_inmemory_join(
    parent: Expression,
    nested: Vec<ReadQuery>,
    builder: &dyn QueryBuilder,
) -> TranslateResult<Expression> {
    let all_linking_fields = nested
        .iter()
        .flat_map(|nested| match nested {
            ReadQuery::RelatedRecordsQuery(rrq) => rrq.parent_field.left_scalars(),
            _ => unreachable!(),
        })
        .unique()
        .sorted_by(|a, b| a.name().cmp(b.name()));

    let linking_fields_bindings = all_linking_fields
        .map(|sf| Binding {
            name: binding::join_parent_field(&sf),
            expr: Expression::MapField {
                field: sf.db_name().into(),
                records: Box::new(Expression::Get {
                    name: binding::join_parent(),
                }),
            },
        })
        .collect();

    let can_assume_strict_equality = nested
        .iter()
        .all(|nested| nested.model().dm.schema.connector.can_assume_strict_equality_in_joins());
    let join_expressions = nested
        .into_iter()
        .filter_map(|nested| match nested {
            ReadQuery::RelatedRecordsQuery(rrq) => Some(rrq),
            _ => None,
        })
        .map(|rrq| -> TranslateResult<JoinExpression> {
            let has_unique_parent = !parent.r#type().is_list();
            let prefixed_parent_field_name = binding::nested_relation_field(&rrq.parent_field);
            let left_scalars = rrq.parent_field.left_scalars();

            let links = left_scalars
                .iter()
                .zip(get_relation_scalars_for_filters(&rrq.parent_field))
                .map(|(parent_scalar, child_scalar)| {
                    let placeholder = Placeholder {
                        name: binding::join_parent_field(parent_scalar),
                        r#type: parent_scalar.type_info().to_prisma_type(),
                    };
                    let condition = if has_unique_parent {
                        ScalarCondition::Equals(ConditionValue::value(PrismaValue::from(placeholder)))
                    } else {
                        ScalarCondition::In(placeholder.into())
                    };
                    ConditionalLink::new(child_scalar.clone(), vec![condition])
                })
                .collect();
            let (child, join) = build_read_related_records(rrq, links, has_unique_parent, builder)?;

            Ok(JoinExpression {
                child,
                is_relation_unique: join.is_relation_unique,
                on: left_scalars
                    .into_iter()
                    .map(|sf| sf.db_name().into())
                    .zip(join.into_fields())
                    .collect(),
                parent_field: prefixed_parent_field_name.into_owned(),
            })
        })
        .try_collect()?;

    Ok(Expression::Let {
        bindings: vec![Binding {
            name: binding::join_parent(),
            expr: parent,
        }],
        expr: Box::new(Expression::Let {
            bindings: linking_fields_bindings,
            expr: Box::new(Expression::Join {
                parent: Box::new(Expression::Get {
                    name: binding::join_parent(),
                }),
                children: join_expressions,
                can_assume_strict_equality,
            }),
        }),
    })
}

fn build_read_related_records(
    mut rrq: RelatedRecordsQuery,
    links: Vec<ConditionalLink>,
    has_unique_parent: bool,
    builder: &dyn QueryBuilder,
) -> TranslateResult<(Expression, JoinMetadata)> {
    // Skip the query entirely if the take is 0.
    if rrq.args.take == Take::Some(0) {
        return Ok((Expression::Concat(vec![]), JoinMetadata::default()));
    }

    let mut linkage = RelationLinkage::new(rrq.parent_field.clone(), links);

    if let Some(results) = rrq.parent_results {
        let parent_link_id = rrq.parent_field.linking_fields();
        let selection = results
            .into_iter()
            .exactly_one()
            .expect("parent results should be exactly one in the query compiler")
            .split_into(slice::from_ref(&parent_link_id))
            .pop()
            .unwrap();

        for (field, val) in FieldSelection::from(get_relation_scalars_for_filters(&rrq.parent_field))
            .assimilate(selection)
            .map_err(QueryGraphBuilderError::from)?
            .pairs
            .into_iter()
        {
            let Some(sf) = field.as_scalar() else { continue };
            let p = val.into_placeholder().expect("expected placeholder in parent results");
            linkage.add_condition(sf.clone(), ScalarCondition::In(p.into()));
        }
    }

    let selected_fields = rrq.selected_fields.without_relations().into_virtuals_last();

    let mut in_memory_ops =
        in_memory_processing::extract_in_memory_ops_for_nested_query(&mut rrq.args, has_unique_parent);

    let (mut child_query, join) = if rrq.parent_field.relation().is_many_to_many() {
        build_read_m2m_query(linkage, rrq.args, &selected_fields, builder)?
    } else {
        build_read_one2m_query(linkage, rrq.args, &selected_fields, builder)?
    };

    in_memory_ops.linking_fields = Some(join.fields.clone());

    child_query = in_memory_ops.into_expression(child_query);

    if !rrq.nested.is_empty() {
        child_query = add_inmemory_join(child_query, rrq.nested, builder)?;
    };

    Ok((child_query, join))
}

/// Returns the scalar fields that would be used to filter the children by. The returned fields
/// do not necessarily represent the actual SQL filter, since some of the underlying SQL fields
/// cannot be represented within our data model (for example m2m linking fields). This function
/// is primarily useful for inferring the correct types of parameters.
///
/// For one-to-one and one-to-many relations, this function returns the linking fields of the child
/// model. It is not correct to do the same for many-to-many relations though, because for them
/// the linking fields of the child do not link to the parent model's identifiers, but rather to
/// the linking table. Instead, we return the linking fields of the parent model, since that's
/// what would be used to query the linking table.
fn get_relation_scalars_for_filters(rf: &RelationField) -> Vec<ScalarField> {
    if rf.relation().is_many_to_many() {
        rf.left_scalars()
    } else {
        rf.related_field().left_scalars()
    }
}

fn build_read_m2m_query(
    linkage: RelationLinkage,
    args: QueryArguments,
    selected_fields: &FieldSelection,
    builder: &dyn QueryBuilder,
) -> TranslateResult<(Expression, JoinMetadata)> {
    let result = builder
        .build_get_related_records(linkage, args, selected_fields)
        .map_err(TranslateError::QueryBuildFailure)?;

    Ok((
        Expression::Query(result.query),
        JoinMetadata {
            fields: vec![result.linking_field_alias],
            is_relation_unique: false,
        },
    ))
}

fn build_read_one2m_query(
    linkage: RelationLinkage,
    mut args: QueryArguments,
    selected_fields: &FieldSelection,
    builder: &dyn QueryBuilder,
) -> TranslateResult<(Expression, JoinMetadata)> {
    let (field, conditions_per_field) = linkage.into_parent_field_and_conditions();

    let filters = args
        .filter
        .take()
        .into_iter()
        .chain(conditions_per_field.flat_map(|(field, conditions)| {
            conditions.into_iter().map(move |condition| {
                Filter::Scalar(ScalarFilter {
                    condition,
                    projection: ScalarProjection::Single(field.clone()),
                    mode: QueryMode::Default,
                })
            })
        }))
        .collect_vec();

    args.filter = Some(Filter::And(filters));

    let expr = build_get_records(
        builder,
        &field.related_model(),
        args,
        selected_fields,
        RelationLoadStrategy::Query,
    )?;

    Ok((
        expr,
        JoinMetadata {
            fields: field
                .related_field()
                .left_scalars()
                .iter()
                .map(|sf| sf.db_name().into())
                .collect(),
            is_relation_unique: !field.arity().is_list(),
        },
    ))
}

fn build_get_records(
    builder: &dyn QueryBuilder,
    model: &Model,
    args: QueryArguments,
    selected_fields: &FieldSelection,
    relation_load_strategy: RelationLoadStrategy,
) -> Result<Expression, TranslateError> {
    Ok(builder
        .build_get_records(model, args, selected_fields, relation_load_strategy)
        .map_err(TranslateError::QueryBuildFailure)?
        .into_iter()
        .map(Expression::Query)
        .reduce(|acc, q| match acc {
            Expression::Concat(mut vec) => {
                vec.push(q);
                Expression::Concat(vec)
            }
            _ => Expression::Concat(vec![acc, q]),
        })
        .expect("should always have at least one query"))
}

fn convert_options_to_validation(expr: Expression, options: QueryOptions) -> Expression {
    if options.contains(QueryOption::ThrowOnEmpty) {
        let expectation =
            DataExpectation::non_empty_rows(MissingRecord::builder().operation(DataOperation::Query).build());
        Expression::validate_expectation(&expectation, expr)
    } else {
        expr
    }
}

#[derive(Debug, Default, Clone)]
struct JoinMetadata {
    fields: Vec<String>,
    is_relation_unique: bool,
}

impl JoinMetadata {
    fn into_fields(self) -> Vec<String> {
        self.fields
    }
}
