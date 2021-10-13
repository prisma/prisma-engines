pub mod read;
pub mod write;

pub use read::*;
pub use write::*;

use crate::model_extensions::RecordProjectionExt;
use prisma_models::RecordProjection;
use quaint::ast::{Column, Comparable, ConditionTree, Query, Row, Values};

const PARAMETER_LIMIT: usize = 2000;

#[tracing::instrument(skip(columns, records, f))]
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
            let tree = conditions(columns, chunk.iter().copied());
            f(tree).into()
        })
        .collect()
}

#[tracing::instrument(skip(columns, records))]
pub(super) fn conditions<'a>(
    columns: &'a [Column<'static>],
    records: impl IntoIterator<Item = &'a RecordProjection>,
) -> ConditionTree<'static> {
    let mut values = Values::empty();

    for proj in records.into_iter() {
        let vals: Vec<_> = proj.db_values();
        values.push(vals)
    }

    Row::from(columns.to_vec()).in_selection(values).into()
}
