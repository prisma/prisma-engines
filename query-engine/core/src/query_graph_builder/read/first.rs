use prisma_models::ModelRef;

use super::*;
use crate::ParsedField;

pub fn find_first(field: ParsedField, model: ModelRef) -> QueryGraphBuilderResult<ReadQuery> {
    let mut many_query = many::find_many(field, model)?;

    // Optimization: Add `take: 1` to the query to reduce fetched result set size if possible.
    Ok(match many_query {
        ReadQuery::ManyRecordsQuery(ref mut m) if m.args.take.is_none() => {
            m.args.take = Some(1);
            many_query
        }
        _ => many_query,
    })
}
