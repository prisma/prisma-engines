use super::*;
use crate::{
    query_document::ParsedField, AggregateRecordsQuery, AggregationType, ArgumentListLookup, FieldPair,
    ParsedInputValue, ReadQuery,
};
use connector::Aggregator;
use prisma_models::{ModelRef, ScalarFieldRef};

pub fn group_by(mut field: ParsedField, model: ModelRef) -> QueryGraphBuilderResult<ReadQuery> {
    let name = field.name;
    let alias = field.alias;
    let model = model;

    let by_argument = field.arguments.lookup("by").unwrap();
    let aggregators = extract_aggregators(&model, by_argument.value)?;

    let args = extractors::extract_query_args(field.arguments, &model)?;
    let nested_fields = field.nested_fields.unwrap().fields;
    let selection_order = todo!();

    // Todo: Generate nested selection based on the grouping. Ordering of fields is best-effort based on occurrence.

    Ok(ReadQuery::AggregateRecordsQuery(AggregateRecordsQuery {
        name,
        alias,
        model,
        selection_order,
        args,
        typ: AggregationType::GroupBy(aggregators),
    }))
}

fn extract_aggregators(model: &ModelRef, value: ParsedInputValue) -> QueryGraphBuilderResult<Vec<Aggregator>> {
    match value {
        ParsedInputValue::Map(mut map) => {
            let field = map.remove("field").expect("Validation must guarantee that ");
            todo!()
        }
        ParsedInputValue::List(list) => list
            .into_iter()
            .map(|item| extract_aggregators(model, item))
            .collect::<QueryGraphBuilderResult<Vec<_>>>()
            .map(|lists| lists.into_iter().flatten().collect()),
        _ => {
            return Err(QueryGraphBuilderError::InputError(
                "Expected parsing to guarantee either an object or list is provided for group by.".to_owned(),
            ))
        }
    }
}
