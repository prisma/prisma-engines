pub mod read;
pub mod write;

pub use read::*;
pub use write::*;

use prisma_models::RecordProjection;
use quaint::ast::{Column, Comparable, ConditionTree, Query, Row, Values};

const PARAMETER_LIMIT: usize = 10000;

pub(super) fn chunked_conditions<F, Q>(
    columns: &[Column<'static>],
    records: &[&RecordProjection],
    f: F,
) -> Vec<Query<'static>>
where
    Q: Into<Query<'static>>,
    F: Fn(ConditionTree<'static>) -> Q,
{
    records
        .chunks(PARAMETER_LIMIT)
        .map(|chunk| {
            let tree = conditions(columns, chunk.into_iter().map(|r| *r));
            f(tree).into()
        })
        .collect()
}

pub(super) fn conditions<'a>(
    columns: &'a [Column<'static>],
    records: impl IntoIterator<Item = &'a RecordProjection>,
) -> ConditionTree<'static> {
    let mut values = Values::new();

    for proj in records.into_iter() {
        let vals: Vec<_> = proj.values().collect();
        values.push(vals)
    }

    Row::from(columns.to_vec()).in_selection(values).into()
}
