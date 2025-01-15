mod read;
mod write;

use query_builder::QueryBuilder;
use read::translate_read_query;
use write::translate_write_query;

use crate::{compiler::expression::Expression, Query};

use super::TranslateResult;

pub(crate) fn translate_query(query: Query, builder: &dyn QueryBuilder) -> TranslateResult<Expression> {
    match query {
        Query::Read(rq) => translate_read_query(rq, builder),
        Query::Write(wq) => translate_write_query(wq, builder),
    }
}
