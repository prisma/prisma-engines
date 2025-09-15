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
    match query {
        ReadQuery::ManyRecordsQuery(ref mut m) => {
            m.args.take = match m.args.take {
                Take::All | Take::Some(1) => Take::One,
                Take::Some(-1) => Take::Some(-1),
                _ => {
                    return Err(QueryGraphBuilderError::InputError(
                        "The 'findFirst' operation cannot be used with a 'take' argument that isn't 1 or -1".into(),
                    ));
                }
            };
            Ok(query)
        }
        _ => Ok(query),
    }
}
