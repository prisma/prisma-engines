pub mod column_metadata;
mod context;
mod cursor_condition;
mod filter;
mod join_utils;
pub mod limit;
mod model_extensions;
mod nested_aggregations;
mod ordering;
mod query_arguments_ext;
pub mod read;
#[cfg(feature = "relation_joins")]
pub mod select;
mod sql_trace;
pub mod write;

use quaint::ast::{Column, Comparable, ConditionTree, Query, Row, Values};
use query_structure::SelectionResult;

pub use column_metadata::ColumnMetadata;
pub use context::Context;
pub use filter::FilterBuilder;
pub use model_extensions::{AsColumn, AsColumns, AsTable, RelationFieldExt, SelectionResultExt};
pub use query_arguments_ext::QueryArgumentsExt;
pub use sql_trace::SqlTraceComment;

const PARAMETER_LIMIT: usize = 2000;

pub fn chunked_conditions<F, Q>(
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

pub fn in_conditions<'a>(
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
