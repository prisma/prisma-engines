use std::convert::TryInto;

use super::*;
use crate::{query_document::ParsedField, AggregateRecordsQuery, ArgumentListLookup, ParsedInputValue, ReadQuery};
use prisma_models::{ModelRef, OrderBy, ScalarFieldRef};

pub fn group_by(mut field: ParsedField, model: ModelRef) -> QueryGraphBuilderResult<ReadQuery> {
    let name = field.name;
    let alias = field.alias;
    let model = model;

    let by_arg = field.arguments.lookup("by").unwrap().value;
    let group_by = extract_grouping(by_arg)?;

    let args = extractors::extract_query_args(field.arguments, &model)?;
    let nested_fields = field.nested_fields.unwrap().fields;
    let selection_order = collect_selection_tree(&nested_fields);

    let selectors: Vec<_> = nested_fields
        .into_iter()
        .map(|field| resolve_query(field, &model))
        .collect::<QueryGraphBuilderResult<_>>()?;

    verify_selections(&selectors, &group_by).and_then(|_| verify_orderings(&args.order_by, &group_by))?;

    Ok(ReadQuery::AggregateRecordsQuery(AggregateRecordsQuery {
        name,
        alias,
        model,
        selection_order,
        args,
        selectors,
        group_by,
    }))
}

/// Cross checks that the selections of the request are valid with regard to the requested group bys.
/// Rules:
/// - Every plain scalar field in the selectors must be present in the group by as well.
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

fn extract_grouping(value: ParsedInputValue) -> QueryGraphBuilderResult<Vec<ScalarFieldRef>> {
    match value {
        ParsedInputValue::ScalarField(field) => Ok(vec![field]),

        ParsedInputValue::List(list) => list
            .into_iter()
            .map(|item| Ok(item.try_into()?))
            .collect::<QueryGraphBuilderResult<Vec<ScalarFieldRef>>>(),

        _ => {
            return Err(QueryGraphBuilderError::InputError(
                "Expected parsing to guarantee either a single enum or list a list of enums is provided for group by `by` arg.".to_owned(),
            ))
        }
    }
}
