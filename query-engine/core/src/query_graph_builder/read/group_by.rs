use std::convert::TryInto;

use super::*;
use crate::{
    query_document::ParsedField, AggregateRecordsQuery, ArgumentListLookup, FieldPair, ParsedInputValue, ReadQuery,
};
use connector::AggregationSelection;
use prisma_models::{ModelRef, PrismaValue, ScalarFieldRef};

pub fn group_by(mut field: ParsedField, model: ModelRef) -> QueryGraphBuilderResult<ReadQuery> {
    let name = field.name;
    let alias = field.alias;
    let model = model;

    let by_arg = field.arguments.lookup("by").unwrap().value;
    let group_by = extract_grouping(&model, by_arg)?;

    let args = extractors::extract_query_args(field.arguments, &model)?;
    let nested_fields = field.nested_fields.unwrap().fields;
    let selection_order = vec![];

    let selectors = vec![];

    // Todo: Generate nested selection based on the grouping. Ordering of fields is best-effort based on occurrence.

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

fn extract_grouping(model: &ModelRef, value: ParsedInputValue) -> QueryGraphBuilderResult<Vec<ScalarFieldRef>> {
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

fn extract_selections(model: &ModelRef, value: ParsedInputValue) -> QueryGraphBuilderResult<Vec<AggregationSelection>> {
    match value {
        ParsedInputValue::Map(mut map) => {
            let field: ScalarFieldRef = map
                .remove("field")
                .expect("Validation must guarantee that ")
                .try_into()?;

            Ok(map
                .remove("operation")
                .map(|op| {
                    let op: PrismaValue = op.try_into().unwrap();
                    let field = field.clone();
                    let selection = match op.into_string().unwrap().as_str() {
                        "count" => AggregationSelection::Count(None),
                        "avg" => AggregationSelection::Average(vec![field]),
                        "sum" => AggregationSelection::Sum(vec![field]),
                        "min" => AggregationSelection::Min(vec![field]),
                        "max" => AggregationSelection::Max(vec![field]),
                        _ => unreachable!(),
                    };

                    vec![selection]
                })
                .unwrap_or_else(|| vec![AggregationSelection::Field(field)]))
        }
        ParsedInputValue::List(list) => list
            .into_iter()
            .map(|item| extract_selections(model, item))
            .collect::<QueryGraphBuilderResult<Vec<_>>>()
            .map(|lists| lists.into_iter().flatten().collect()),
        _ => {
            return Err(QueryGraphBuilderError::InputError(
                "Expected parsing to guarantee either an object or list is provided for group by.".to_owned(),
            ))
        }
    }
}
