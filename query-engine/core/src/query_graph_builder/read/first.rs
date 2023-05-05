use prisma_models::ModelRef;

use super::*;
use crate::ParsedField;

pub(crate) fn find_first(field: ParsedField<'_>, model: ModelRef) -> QueryGraphBuilderResult<ReadQuery> {
    let many_query = many::find_many(field, model)?;
    try_limit_to_one(many_query)
}

pub(crate) fn find_first_or_throw(field: ParsedField<'_>, model: ModelRef) -> QueryGraphBuilderResult<ReadQuery> {
    let many_query = many::find_many_or_throw(field, model)?;
    try_limit_to_one(many_query)
}

#[inline]
fn try_limit_to_one(mut query: ReadQuery) -> QueryGraphBuilderResult<ReadQuery> {
    Ok(match query {
        ReadQuery::ManyRecordsQuery(ref mut m) if m.args.take.is_none() => {
            m.args.take = Some(1);
            query
        }
        _ => query,
    })
}
