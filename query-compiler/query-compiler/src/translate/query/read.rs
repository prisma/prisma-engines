use crate::{
    TranslateError, binding,
    expression::{Binding, Expression, JoinExpression, Pagination},
    translate::TranslateResult,
};
use itertools::Itertools;
use query_builder::{ConditionalLink, QueryArgumentsExt, QueryBuilder, RelationLinkage};
use query_core::{
    AggregateRecordsQuery, DataExpectation, DataOperation, MissingRecord, QueryGraphBuilderError, QueryOption,
    QueryOptions, ReadQuery, RelatedRecordsQuery,
};
use query_structure::{
    ConditionValue, FieldSelection, Filter, Model, PrismaValue, QueryArguments, QueryMode, RelationLoadStrategy,
    ScalarCondition, ScalarFilter, ScalarProjection, Take,
};
use std::slice;

pub(crate) fn translate_read_query(query: ReadQuery, builder: &dyn QueryBuilder) -> TranslateResult<Expression> {
    Ok(match query {
        ReadQuery::RecordQuery(rq) => {
            let selected_fields = match rq.relation_load_strategy {
                RelationLoadStrategy::Join => rq.selected_fields.into_virtuals_last(),
                RelationLoadStrategy::Query => rq.selected_fields.without_relations().into_virtuals_last(),
            };

            let args = QueryArguments::from((
                rq.model.clone(),
                rq.filter.expect("ReadOne query should always have filter set"),
            ))
            .with_take(Take::One);

            let expr = build_get_records(builder, &rq.model, args, &selected_fields, rq.relation_load_strategy)?;
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

            let needs_reversed_order = mrq.args.needs_reversed_order();
            let take = mrq.args.take;

            let pagination = mrq
                .args
                .requires_inmemory_processing()
                .then(|| extract_pagination(&mut mrq.args));
            let distinct_by = mrq
                .args
                .requires_inmemory_distinct()
                .then(|| extract_distinct_by(&mut mrq.args));

            let mut expr = build_get_records(
                builder,
                &mrq.model,
                mrq.args,
                &selected_fields,
                mrq.relation_load_strategy,
            )?;

            if let Some(fields) = distinct_by {
                expr = Expression::DistinctBy {
                    expr: expr.into(),
                    fields,
                };
            };

            if let Some(pagination) = pagination {
                expr = Expression::Paginate {
                    expr: expr.into(),
                    pagination,
                };
            };

            if needs_reversed_order {
                expr = Expression::Reverse(Box::new(expr));
            };

            expr = convert_options_to_validation(expr, mrq.options);

            if mrq.relation_load_strategy == RelationLoadStrategy::Query && !mrq.nested.is_empty() {
                expr = add_inmemory_join(expr, mrq.nested, builder)?;
            }

            match take {
                Take::One => Expression::Unique(Box::new(expr)),
                _ => expr,
            }
        }

        ReadQuery::RelatedRecordsQuery(rrq) => {
            let (expr, join) = build_read_related_records(rrq, vec![], builder)?;
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

    let join_expressions = nested
        .into_iter()
        .filter_map(|nested| match nested {
            ReadQuery::RelatedRecordsQuery(rrq) => Some(rrq),
            _ => None,
        })
        .map(|rrq| -> TranslateResult<JoinExpression> {
            let prefixed_parent_field_name = binding::nested_relation_field(&rrq.parent_field);
            let left_scalars = rrq.parent_field.left_scalars();
            let links = left_scalars
                .iter()
                .zip(rrq.parent_field.related_field().left_scalars())
                .map(|(parent_scalar, child_scalar)| {
                    let placeholder = PrismaValue::placeholder(
                        binding::join_parent_field(parent_scalar),
                        parent_scalar.type_info().to_prisma_type(),
                    );
                    let condition = if parent.r#type().is_list() {
                        ScalarCondition::InTemplate(ConditionValue::value(placeholder))
                    } else {
                        ScalarCondition::Equals(ConditionValue::value(placeholder))
                    };
                    ConditionalLink::new(child_scalar.clone(), vec![condition])
                })
                .collect();
            let (child, join) = build_read_related_records(rrq, links, builder)?;

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
            }),
        }),
    })
}

fn build_read_related_records(
    mut rrq: RelatedRecordsQuery,
    links: Vec<ConditionalLink>,
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

        // When we query for children by parent, we typically want to generate a query with filters
        // for every field in `parent_field.related_field().linking_fields()`. It's not correct to
        // do that for many-to-many relations though, because their `related_field` points at the
        // primary identifier of the child model, which cannot be used as a filter for the parent
        // identifiers. The actual field that must be used belongs to the linking table, and it
        // corresponds to the primary identifier of the parent model.
        let fields_to_filter_by = if rrq.parent_field.relation().is_many_to_many() {
            parent_link_id
        } else {
            rrq.parent_field.related_field().linking_fields()
        };

        for (field, val) in fields_to_filter_by
            .assimilate(selection)
            .map_err(QueryGraphBuilderError::from)?
            .pairs
            .into_iter()
        {
            let Some(sf) = field.as_scalar() else { continue };
            linkage.add_condition(sf.clone(), ScalarCondition::InTemplate(val.into()));
        }
    }

    let selected_fields = rrq.selected_fields.without_relations().into_virtuals_last();
    let needs_reversed_order = rrq.args.needs_reversed_order();

    let pagination = (rrq.args.take.is_some() || rrq.args.skip.is_some() || rrq.args.cursor.is_some())
        .then(|| extract_pagination(&mut rrq.args));
    let distinct_by = (rrq.args.distinct.is_some()).then(|| extract_distinct_by(&mut rrq.args));

    let (mut child_query, join) = if rrq.parent_field.relation().is_many_to_many() {
        build_read_m2m_query(linkage, rrq.args, &selected_fields, builder)?
    } else {
        build_read_one2m_query(linkage, rrq.args, &selected_fields, builder)?
    };

    if let Some(fields) = distinct_by {
        child_query = Expression::DistinctBy {
            expr: child_query.into(),
            fields: fields.into_iter().chain(join.fields.iter().cloned()).collect(),
        };
    };

    if let Some(pagination) = pagination {
        child_query = Expression::Paginate {
            expr: child_query.into(),
            pagination: pagination.with_linking_fields(join.fields.clone()),
        };
    };

    if needs_reversed_order {
        child_query = Expression::Reverse(Box::new(child_query));
    };

    if !rrq.nested.is_empty() {
        child_query = add_inmemory_join(child_query, rrq.nested, builder)?;
    };

    Ok((child_query, join))
}

fn build_read_m2m_query(
    linkage: RelationLinkage,
    args: QueryArguments,
    selected_fields: &FieldSelection,
    builder: &dyn QueryBuilder,
) -> TranslateResult<(Expression, JoinMetadata)> {
    let link_name = linkage.to_string();

    let query = builder
        .build_get_related_records(linkage, args, selected_fields)
        .map_err(TranslateError::QueryBuildFailure)?;

    Ok((
        Expression::Query(query),
        JoinMetadata {
            fields: vec![link_name],
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

fn extract_pagination(args: &mut QueryArguments) -> Pagination {
    args.ignore_take = true;
    args.ignore_skip = true;

    let cursor = args.cursor.as_ref().map(|cursor| {
        cursor
            .pairs()
            .map(|(sf, val)| (sf.db_name().into_owned(), val.clone()))
            .collect()
    });
    Pagination::builder()
        .maybe_cursor(cursor)
        .maybe_take(args.take.abs())
        .maybe_skip(args.skip)
        .build()
}

fn extract_distinct_by(args: &mut QueryArguments) -> Vec<String> {
    let distinct = args.distinct.take().unwrap();
    distinct.db_names().collect_vec()
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
