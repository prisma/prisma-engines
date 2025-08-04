use itertools::Itertools;
use query_builder::QueryArgumentsExt;
use query_core::ReadQuery;
use query_structure::{QueryArguments, RelationLoadStrategy};

use crate::expression::{InMemoryOps, Pagination};

pub fn extract_in_memory_ops(
    args: &mut QueryArguments,
    rls: RelationLoadStrategy,
    nested: &mut [ReadQuery],
) -> InMemoryOps {
    InMemoryOps::builder()
        .reverse(args.needs_reversed_order())
        .maybe_pagination(args.requires_inmemory_pagination(rls).then(|| extract_pagination(args)))
        .maybe_distinct(args.requires_inmemory_distinct(rls).then(|| extract_distinct_by(args)))
        .maybe_nested((rls == RelationLoadStrategy::Join).then(|| {
            nested
                .iter_mut()
                .filter_map(|rq| match rq {
                    ReadQuery::RelatedRecordsQuery(rrq) => Some(rrq),
                    _ => None,
                })
                .filter_map(|rrq| {
                    let nested_ops = extract_in_memory_ops(&mut rrq.args, rls, &mut rrq.nested);
                    (!nested_ops.is_empty()).then(|| (rrq.parent_field.name().to_owned(), nested_ops))
                })
                .collect()
        }))
        .build()
}

pub fn extract_in_memory_ops_for_nested_query(args: &mut QueryArguments, has_unique_parent: bool) -> InMemoryOps {
    // We are forced to use in-memory processing when we have potentially more than one parent
    // record (!has_unique_parent). Otherwise our skip/limit on the database level would apply to
    // children of multiple parents, which would produce incorrect results.
    let needs_pagination = args.take.is_some() || args.skip.is_some() || args.cursor.is_some();
    let must_paginate_in_memory =
        needs_pagination && (!has_unique_parent || args.requires_inmemory_processing(RelationLoadStrategy::Query));

    let needs_distinct = args.distinct.is_some();
    let must_distinct_in_memory =
        needs_distinct && (!has_unique_parent || args.requires_inmemory_distinct(RelationLoadStrategy::Query));

    InMemoryOps::builder()
        .reverse(args.needs_reversed_order())
        .maybe_pagination(must_paginate_in_memory.then(|| extract_pagination(args)))
        .maybe_distinct(must_distinct_in_memory.then(|| extract_distinct_by(args)))
        .build()
}

fn extract_pagination(args: &mut QueryArguments) -> Pagination {
    args.ignore_take = true;
    args.ignore_skip = true;

    let cursor = args.cursor.as_ref().map(|cursor| {
        cursor
            .pairs()
            .map(|(sf, val)| (sf.db_name().into_owned(), val.clone()))
            .collect()
    });

    Pagination::builder()
        .maybe_cursor(cursor)
        .maybe_take(args.take.abs())
        .maybe_skip(args.skip)
        .build()
}

fn extract_distinct_by(args: &mut QueryArguments) -> Vec<String> {
    let distinct = args.distinct.take().unwrap();
    distinct.db_names().collect_vec()
}
