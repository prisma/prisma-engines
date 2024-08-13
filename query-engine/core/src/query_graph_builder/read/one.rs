use super::{utils::get_relation_load_strategy, *};
use crate::{query_document::*, QueryOption, QueryOptions, ReadQuery, RecordQuery};
use query_structure::Model;
use schema::{constants::args, QuerySchema};
use std::convert::TryInto;

pub(crate) fn find_unique(
    field: ParsedField<'_>,
    model: Model,
    query_schema: &QuerySchema,
) -> QueryGraphBuilderResult<ReadQuery> {
    find_unique_with_options(field, model, QueryOptions::none(), query_schema)
}

pub(crate) fn find_unique_or_throw(
    field: ParsedField<'_>,
    model: Model,
    query_schema: &QuerySchema,
) -> QueryGraphBuilderResult<ReadQuery> {
    find_unique_with_options(field, model, QueryOption::ThrowOnEmpty.into(), query_schema)
}

/// Builds a read query from a parsed incoming read query field.
#[inline]
fn find_unique_with_options(
    mut field: ParsedField<'_>,
    model: Model,
    options: QueryOptions,
    query_schema: &QuerySchema,
) -> QueryGraphBuilderResult<ReadQuery> {
    let filter = match field.arguments.lookup(args::WHERE) {
        Some(where_arg) => {
            let arg: ParsedInputMap<'_> = where_arg.value.try_into()?;
            Some(extractors::extract_unique_filter(arg, &model)?)
        }
        None => None,
    };

    let requested_rel_load_strategy = field
        .arguments
        .lookup(args::RELATION_LOAD_STRATEGY)
        .map(|arg| arg.value.try_into())
        .transpose()?;

    let name = field.name;
    let alias = field.alias;
    let (selected_fields, selection_order, nested) =
        utils::extract_selected_fields(field.nested_fields.unwrap().fields, &model, query_schema)?;

    let relation_load_strategy = get_relation_load_strategy(requested_rel_load_strategy, None, &nested, query_schema)?;

    Ok(ReadQuery::RecordQuery(RecordQuery {
        name,
        alias,
        model,
        filter,
        selected_fields,
        nested,
        selection_order,
        options,
        relation_load_strategy,
    }))
}
