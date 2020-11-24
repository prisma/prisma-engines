use std::convert::TryInto;

use super::*;
use crate::{query_document::ParsedField, AggregateRecordsQuery, ArgumentListLookup, ParsedInputValue, ReadQuery};
use prisma_models::{ModelRef, ScalarFieldRef};

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

    // Todo cross verify selections and group by

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
