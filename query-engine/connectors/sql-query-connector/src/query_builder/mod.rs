pub mod read;
pub mod write;

pub use read::*;
pub use write::*;

use quaint::ast::{Column, Query, ConditionTree, Comparable, Conjuctive};
use prisma_models::RecordIdentifier;

const PARAMETER_LIMIT: usize = 10000;

pub(super) fn chunked_conditions<F, Q>(
    columns: &[Column<'static>],
    records: &[&RecordIdentifier],
    f: F,
) -> Vec<Query<'static>>
where
    Q: Into<Query<'static>>,
    F: Fn(ConditionTree<'static>) -> Q,
{
    records.chunks(PARAMETER_LIMIT).map(|chunk| {
        let tree = conditions(columns, chunk.into_iter().map(|r| *r));
        f(tree).into()
    }).collect()
}

pub(super) fn conditions<'a>(
    columns: &'a [Column<'static>],
    records: impl IntoIterator<Item = &'a RecordIdentifier>,
) -> ConditionTree<'static>
{
    records.into_iter().map(|ids| {
        let cols_with_vals = columns.into_iter().map(|c| c.clone()).zip(ids.values());

        cols_with_vals.fold(ConditionTree::NoCondition, |acc, (col, val)| {
            match acc {
                ConditionTree::NoCondition => col.equals(val).into(),
                cond => cond.and(col.equals(val)),

            }
        })
    }).fold(ConditionTree::NoCondition, |acc, cond| {
        match acc {
            ConditionTree::NoCondition => cond,
            acc => acc.or(cond),
        }
    })
}
