use query_structure::{Model, Take};
use schema::QuerySchema;

use super::*;
use crate::ParsedField;

pub(crate) fn find_first(
    field: ParsedField<'_>,
    model: Model,
    query_schema: &QuerySchema,
) -> QueryGraphBuilderResult<ReadQuery> {
    let many_query = many::find_many(field, model, query_schema)?;
    try_limit_to_one(many_query)
}

pub(crate) fn find_first_or_throw(
    field: ParsedField<'_>,
    model: Model,
    query_schema: &QuerySchema,
) -> QueryGraphBuilderResult<ReadQuery> {
    let many_query = many::find_many_or_throw(field, model, query_schema)?;
    try_limit_to_one(many_query)
}

#[inline]
fn try_limit_to_one(mut query: ReadQuery) -> QueryGraphBuilderResult<ReadQuery> {
    Ok(match query {
        ReadQuery::ManyRecordsQuery(ref mut m) if m.args.take.is_all() => {
            m.args.take = Take::One;
            query
        }
        _ => query,
    })
}
