use crate::{
    TranslateError,
    expression::{Binding, Expression, JoinExpression, Pagination},
    translate::TranslateResult,
};
use itertools::Itertools;
use query_builder::{ConditionalLink, JoinLinks, QueryArgumentsExt, QueryBuilder};
use query_core::{
    AggregateRecordsQuery, DataExpectation, DataOperation, FilteredQuery, MissingRecord, QueryGraphBuilderError,
    QueryOption, QueryOptions, ReadQuery, RelatedRecordsQuery,
};
use query_structure::{
    ConditionValue, FieldSelection, Filter, PrismaValue, QueryArguments, QueryMode, ScalarCondition, ScalarFilter,
    ScalarProjection, Take,
};
use std::slice;

pub(crate) fn translate_read_query(query: ReadQuery, builder: &dyn QueryBuilder) -> TranslateResult<Expression> {
    Ok(match query {
        ReadQuery::RecordQuery(rq) => {
            let selected_fields = rq.selected_fields.without_relations().into_virtuals_last();

            let args = QueryArguments::from((
                rq.model.clone(),
                rq.filter.expect("ReadOne query should always have filter set"),
            ))
            .with_take(Take::One);
            let query = builder
                .build_get_records(&rq.model, args, &selected_fields)
                .map_err(TranslateError::QueryBuildFailure)?;

            let expr = Expression::Query(query);
            let expr = convert_options_to_validation(expr, rq.options);
            let expr = Expression::Unique(Box::new(expr));

            if rq.nested.is_empty() {
                expr
            } else {
                add_inmemory_join(expr, rq.nested, builder)?
            }
        }

        ReadQuery::ManyRecordsQuery(mut mrq) => {
            let selected_fields = mrq.selected_fields.without_relations().into_virtuals_last();
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

            // TODO: we ignore chunking for now
            let query = builder
                .build_get_records(&mrq.model, mrq.args, &selected_fields)
                .map_err(TranslateError::QueryBuildFailure)?;

            let mut expr = Expression::Query(query);

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

            if !mrq.nested.is_empty() {
                expr = add_inmemory_join(expr, mrq.nested, builder)?;
            };

            match take {
                Take::One => Expression::Unique(Box::new(expr)),
                _ => expr,
            }
        }

        ReadQuery::RelatedRecordsQuery(rrq) => {
            let (expr, _) = build_read_related_records(rrq, vec![], builder)?;
            expr
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
        .unique()
        .sorted_by(|a, b| a.prisma_name().cmp(&b.prisma_name()));

    let linking_fields_bindings = all_linking_fields
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
            let left_scalars = rrq.parent_field.left_scalars();
            let links = left_scalars
                .iter()
                .zip(rrq.parent_field.related_field().left_scalars())
                .map(|(parent_scalar, child_scalar)| {
                    let placeholder = PrismaValue::placeholder(
                        format!("@parent${}", parent_scalar.name()),
                        parent_scalar.type_identifier().to_prisma_type(),
                    );
                    let condition = if parent.r#type().is_list() {
                        ScalarCondition::InTemplate(ConditionValue::value(placeholder))
                    } else {
                        ScalarCondition::Equals(ConditionValue::value(placeholder))
                    };
                    ConditionalLink::new(child_scalar.clone(), vec![condition])
                })
                .collect();
            let (child, join_fields) = build_read_related_records(rrq, links, builder)?;

            Ok(JoinExpression {
                child,
                on: left_scalars
                    .into_iter()
                    .map(|sf| sf.name().to_owned())
                    .zip(join_fields)
                    .collect(),
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

fn build_read_related_records(
    mut rrq: RelatedRecordsQuery,
    links: Vec<ConditionalLink>,
    builder: &dyn QueryBuilder,
) -> TranslateResult<(Expression, JoinFields)> {
    let mut join_links = JoinLinks::new(rrq.parent_field.clone(), links);

    if let Some(results) = rrq.parent_results {
        let parent_link_id = rrq.parent_field.linking_fields();
        let child_link_id = rrq.parent_field.related_field().linking_fields();

        let selection = results
            .into_iter()
            .exactly_one()
            .expect("parent results should be exactly one in the query compiler")
            .split_into(slice::from_ref(&parent_link_id))
            .pop()
            .unwrap();

        for (field, val) in child_link_id
            .assimilate(selection)
            .map_err(QueryGraphBuilderError::from)?
            .pairs
            .into_iter()
        {
            let Some(sf) = field.as_scalar() else { continue };
            join_links.add_condition(sf.clone(), ScalarCondition::InTemplate(val.into()));
        }
    }

    let selected_fields = rrq.selected_fields.without_relations().into_virtuals_last();
    let needs_reversed_order = rrq.args.needs_reversed_order();

    let pagination = (rrq.args.take.is_some() || rrq.args.skip.is_some() || rrq.args.cursor.is_some())
        .then(|| extract_pagination(&mut rrq.args));
    let distinct_by = (rrq.args.distinct.is_some()).then(|| extract_distinct_by(&mut rrq.args));

    let (mut child_query, join_on) = if rrq.parent_field.relation().is_many_to_many() {
        build_read_m2m_query(join_links, rrq.args, &selected_fields, builder)?
    } else {
        build_read_one2m_query(join_links, rrq.args, &selected_fields, builder)?
    };

    if let Some(fields) = distinct_by {
        child_query = Expression::DistinctBy {
            expr: child_query.into(),
            fields: fields.into_iter().chain(join_on.clone()).collect(),
        };
    };

    if let Some(pagination) = pagination {
        child_query = Expression::Paginate {
            expr: child_query.into(),
            pagination: pagination.with_parent_links(join_on.clone()),
        };
    };

    if needs_reversed_order {
        child_query = Expression::Reverse(Box::new(child_query));
    };

    if !rrq.nested.is_empty() {
        child_query = add_inmemory_join(child_query, rrq.nested, builder)?;
    };
    Ok((child_query, join_on))
}

fn build_read_m2m_query(
    join: JoinLinks,
    args: QueryArguments,
    selected_fields: &FieldSelection,
    builder: &dyn QueryBuilder,
) -> TranslateResult<(Expression, JoinFields)> {
    let link_name = join.to_string();

    let query = builder
        .build_get_related_records(join, args, selected_fields)
        .map_err(TranslateError::QueryBuildFailure)?;

    Ok((Expression::Query(query), JoinFields(vec![link_name])))
}

fn build_read_one2m_query(
    join: JoinLinks,
    mut args: QueryArguments,
    selected_fields: &FieldSelection,
    builder: &dyn QueryBuilder,
) -> TranslateResult<(Expression, JoinFields)> {
    let (field, conditions_per_field) = join.into_parent_field_and_conditions();
    let related_scalars = field.related_field().left_scalars();
    let join_fields = related_scalars.iter().map(|sf| sf.name().to_owned()).collect();

    for (field, conditions) in conditions_per_field {
        for condition in conditions {
            args.add_filter(Filter::Scalar(ScalarFilter {
                condition,
                projection: ScalarProjection::Single(field.clone()),
                mode: QueryMode::Default,
            }));
        }
    }

    let to_one_relation = !field.arity().is_list();
    let args = if to_one_relation {
        args.with_take(Take::Some(1))
    } else {
        args
    };
    let query = builder
        .build_get_records(&field.related_model(), args, selected_fields)
        .map_err(TranslateError::QueryBuildFailure)?;

    let mut expr = Expression::Query(query);
    if to_one_relation {
        expr = Expression::Unique(Box::new(expr));
    }
    Ok((expr, JoinFields(join_fields)))
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
    Pagination::new(cursor, args.take.abs(), args.skip)
}

fn extract_distinct_by(args: &mut QueryArguments) -> Vec<String> {
    let distinct = args.distinct.take().unwrap();
    distinct.db_names().collect_vec()
}

#[derive(Debug, Clone)]
struct JoinFields(Vec<String>);

impl From<JoinFields> for Vec<String> {
    fn from(join_fields: JoinFields) -> Self {
        join_fields.0
    }
}

impl IntoIterator for JoinFields {
    type Item = String;
    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}
