pub(crate) mod read;
pub(crate) mod write;

use crate::model_extensions::SelectionResultExt;
use prisma_models::SelectionResult;
use quaint::ast::{Column, Comparable, ConditionTree, Query, Row, Values};

const PARAMETER_LIMIT: usize = 2000;

pub(super) fn chunked_conditions<F, Q>(
    columns: &[Column<'static>],
    records: &[&SelectionResult],
    f: F,
) -> Vec<Query<'static>>
where
    Q: Into<Query<'static>>,
    F: Fn(ConditionTree<'static>) -> Q,
{
    records
        .chunks(PARAMETER_LIMIT)
        .map(|chunk| {
            let tree = in_conditions(columns, chunk.iter().copied());
            f(tree).into()
        })
        .collect()
}

pub(super) fn in_conditions<'a>(
    columns: &'a [Column<'static>],
    results: impl IntoIterator<Item = &'a SelectionResult>,
) -> ConditionTree<'static> {
    let mut values = Values::empty();

    for result in results.into_iter() {
        let vals: Vec<_> = result.db_values();
        values.push(vals)
    }

    Row::from(columns.to_vec()).in_selection(values).into()
}
