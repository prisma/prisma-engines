use super::*;
use crate::{
    query_document::ParsedField, AggregateRecordsQuery, AggregationType, ArgumentListLookup, FieldPair, ReadQuery,
};
use connector::Aggregator;
use prisma_models::{ModelRef, ScalarFieldRef};

pub fn group_by(mut field: ParsedField, model: ModelRef) -> QueryGraphBuilderResult<ReadQuery> {
    let name = field.name;
    let alias = field.alias;
    let model = model;
    let nested_fields = field.nested_fields.unwrap().fields;
    let by_argument = field.arguments.lookup("by").unwrap();
    let selection_order = todo!();

    let args = extractors::extract_query_args(field.arguments, &model)?;

    // Todo: Generate nested selection based on the grouping. Ordering of fields is best-effort based on occurrence.
    let aggregators = todo!();

    Ok(ReadQuery::AggregateRecordsQuery(AggregateRecordsQuery {
        name,
        alias,
        model,
        selection_order,
        args,
        typ: AggregationType::GroupBy(aggregators),
    }))
}
