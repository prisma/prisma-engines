use std::convert::TryInto;

use super::*;
use crate::{
    constants::inputs::args, query_document::ParsedField, AggregateRecordsQuery, ArgumentListLookup, ParsedInputValue,
    ReadQuery,
};
use connector::Filter;
use prisma_models::{ModelRef, OrderBy, ScalarFieldRef};

#[tracing::instrument(skip(field, model))]
pub fn group_by(mut field: ParsedField, model: ModelRef) -> QueryGraphBuilderResult<ReadQuery> {
    let name = field.name;
    let alias = field.alias;
    let model = model;

    let by_arg = field.arguments.lookup(args::BY).unwrap().value;
    let group_by = extract_grouping(by_arg)?;
    let having: Option<Filter> = match field.arguments.lookup(args::HAVING) {
        Some(having_arg) => Some(extract_filter(having_arg.value.try_into()?, &model)?),
        None => None,
    };

    let args = extractors::extract_query_args(field.arguments, &model)?;
    let nested_fields = field.nested_fields.unwrap().fields;
    let selection_order = collect_selection_tree(&nested_fields);

    let selectors: Vec<_> = nested_fields
        .into_iter()
        .map(|field| resolve_query(field, &model))
        .collect::<QueryGraphBuilderResult<_>>()?;

    verify_selections(&selectors, &group_by)
        .and_then(|_| verify_orderings(&args.order_by, &group_by))
        .and_then(|_| verify_having(having.as_ref(), &selectors))?;

    Ok(ReadQuery::AggregateRecordsQuery(AggregateRecordsQuery {
        name,
        alias,
        model,
        selection_order,
        args,
        selectors,
        group_by,
        having,
    }))
}

/// Cross checks that the selections of the request are valid with regard to the requested group bys:
/// Every plain scalar field in the selectors must be present in the group by as well.
fn verify_selections(selectors: &[AggregationSelection], group_by: &[ScalarFieldRef]) -> QueryGraphBuilderResult<()> {
    let mut missing_fields = vec![];

    for selector in selectors {
        if let AggregationSelection::Field(field) = selector {
            if !group_by.contains(&field) {
                missing_fields.push(field.name.clone());
            }
        }
    }

    if missing_fields.is_empty() {
        Ok(())
    } else {
        Err(QueryGraphBuilderError::InputError(format!(
            "Every selected scalar field that is not part of an aggregation \
        must be included in the by-arguments of the query. Missing fields: {}",
            missing_fields.join(", ")
        )))
    }
}

/// Cross checks that the requested order-bys of the request are valid with regard to the requested group bys.
/// Every ordered field must be present in the group by as well. (Note: We do not yet allow order by aggregate)
fn verify_orderings(orderings: &[OrderBy], group_by: &[ScalarFieldRef]) -> QueryGraphBuilderResult<()> {
    let mut missing_fields = vec![];

    for ordering in orderings {
        if !group_by.contains(&ordering.field) {
            missing_fields.push(ordering.field.name.clone());
        }
    }

    if missing_fields.is_empty() {
        Ok(())
    } else {
        Err(QueryGraphBuilderError::InputError(format!(
            "Every field used for orderBy must be included in the by-arguments of the query. Missing fields: {}",
            missing_fields.join(", ")
        )))
    }
}

/// Cross checks that every scalar field used in `having` is either an aggregate or contained in the selectors.
fn verify_having(having: Option<&Filter>, selectors: &[AggregationSelection]) -> QueryGraphBuilderResult<()> {
    if let Some(filter) = having {
        let having_fields: Vec<&ScalarFieldRef> = collect_scalar_fields(filter);
        let selector_fields: Vec<&ScalarFieldRef> = selectors
            .iter()
            .filter_map(|selector| match selector {
                AggregationSelection::Field(field) => Some(field),
                _ => None,
            })
            .collect();

        let missing_fields: Vec<String> = having_fields
            .into_iter()
            .filter_map(|field| {
                if selector_fields.contains(&field) {
                    None
                } else {
                    Some(field.name.clone())
                }
            })
            .collect();

        if missing_fields.is_empty() {
            Ok(())
        } else {
            Err(QueryGraphBuilderError::InputError(format!(
                    "Every field used in `having` filters must either be an aggregation filter or be included in the selection of the query. Missing fields: {}",
                    missing_fields.join(", ")
                )))
        }
    } else {
        Ok(())
    }
}

/// Collects all flat scalar fields that are used in the having filter.
fn collect_scalar_fields(filter: &Filter) -> Vec<&ScalarFieldRef> {
    match filter {
        Filter::And(inner) => inner.iter().flat_map(|f| collect_scalar_fields(f)).collect(),
        Filter::Or(inner) => inner.iter().flat_map(|f| collect_scalar_fields(f)).collect(),
        Filter::Not(inner) => inner.iter().flat_map(|f| collect_scalar_fields(f)).collect(),
        Filter::Scalar(sf) => sf.projection.scalar_fields(),
        Filter::Aggregation(_) => vec![], // Aggregations have no effect here.
        _ => unreachable!(),
    }
}

fn extract_grouping(value: ParsedInputValue) -> QueryGraphBuilderResult<Vec<ScalarFieldRef>> {
    match value {
        ParsedInputValue::ScalarField(field) => Ok(vec![field]),

        ParsedInputValue::List(list) if !list.is_empty() => list
            .into_iter()
            .map(|item| Ok(item.try_into()?))
            .collect::<QueryGraphBuilderResult<Vec<ScalarFieldRef>>>(),

        ParsedInputValue::List(list) if list.is_empty() => Err(QueryGraphBuilderError::InputError(
            "At least one selection is required for the `by` argument.".to_owned(),
        )),

        _ => Err(QueryGraphBuilderError::InputError(
            "Expected parsing to guarantee either a single enum or a list of enums is provided for group by `by` arg."
                .to_owned(),
        )),
    }
}
