pub mod read;
#[cfg(feature = "relation_joins")]
pub mod select;
pub mod write;

use crate::context::Context;
use crate::model_extensions::SelectionResultExt;
use quaint::ast::{Column, Comparable, ConditionTree, Query, Row, Values};
use query_structure::SelectionResult;

const PARAMETER_LIMIT: usize = 2000;

pub(super) fn chunked_conditions<F, Q>(
    columns: &[Column<'static>],
    records: &[&SelectionResult],
    ctx: &Context<'_>,
    f: F,
) -> Vec<Query<'static>>
where
    Q: Into<Query<'static>>,
    F: Fn(ConditionTree<'static>) -> Q,
{
    records
        .chunks(PARAMETER_LIMIT)
        .map(|chunk| {
            let tree = in_conditions(columns, chunk.iter().copied(), ctx);
            f(tree).into()
        })
        .collect()
}

pub(super) fn in_conditions<'a>(
    columns: &'a [Column<'static>],
    results: impl IntoIterator<Item = &'a SelectionResult>,
    ctx: &Context<'_>,
) -> ConditionTree<'static> {
    let mut values = Values::empty();

    for result in results.into_iter() {
        let vals: Vec<_> = result.db_values(ctx);
        values.push(vals)
    }

    Row::from(columns.to_vec()).in_selection(values).into()
}
