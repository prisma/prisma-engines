use super::*;
use crate::{query_document::ParsedField, AggregateRecordsQuery, FieldPair, ReadQuery};
use connector::AggregationSelection;
use prisma_models::{ModelRef, ScalarFieldRef};

pub fn aggregate(field: ParsedField, model: ModelRef) -> QueryGraphBuilderResult<ReadQuery> {
    let name = field.name;
    let alias = field.alias;
    let model = model;
    let nested_fields = field.nested_fields.unwrap().fields;
    let selection_order = collect_selection_tree(&nested_fields);
    let args = extractors::extract_query_args(field.arguments, &model)?;

    // Reject unstable cursors for aggregations, because we can't do post-processing on those (we haven't implemented a in-memory aggregator yet).
    if args.contains_unstable_cursor() {
        return Err(QueryGraphBuilderError::InputError(
            "The chosen cursor and orderBy combination is not stable (unique) and can't be used for aggregations."
                .to_owned(),
        ));
    }

    let selectors: Vec<_> = nested_fields
        .into_iter()
        .map(|field| resolve_query(field, &model))
        .collect::<QueryGraphBuilderResult<_>>()?;

    Ok(ReadQuery::AggregateRecordsQuery(AggregateRecordsQuery {
        name,
        alias,
        model,
        selection_order,
        args,
        selectors,
        group_by: vec![],
    }))
}

/// Resolves the given field as a aggregation query.
fn resolve_query(field: FieldPair, model: &ModelRef) -> QueryGraphBuilderResult<AggregationSelection> {
    let query = match field.parsed_field.name.as_str() {
        "count" => AggregationSelection::Count(None),
        "avg" => AggregationSelection::Average(resolve_fields(model, field)),
        "sum" => AggregationSelection::Sum(resolve_fields(model, field)),
        "min" => AggregationSelection::Min(resolve_fields(model, field)),
        "max" => AggregationSelection::Max(resolve_fields(model, field)),
        _ => unreachable!(),
    };

    Ok(query)
}

fn resolve_fields(model: &ModelRef, field: FieldPair) -> Vec<ScalarFieldRef> {
    let fields = field.parsed_field.nested_fields.unwrap().fields;
    let scalars = model.fields().scalar();

    fields
        .into_iter()
        .map(|f| {
            scalars
                .iter()
                .find_map(|sf| {
                    if sf.name == f.parsed_field.name {
                        Some(sf.clone())
                    } else {
                        None
                    }
                })
                .expect("Expected validation to guarantee valid aggregation fields.")
        })
        .collect()
}

fn collect_selection_tree(fields: &[FieldPair]) -> Vec<(String, Option<Vec<String>>)> {
    fields
        .iter()
        .map(|field| {
            let field = &field.parsed_field;
            (
                field.name.clone(),
                field.nested_fields.as_ref().map(|nested_object| {
                    nested_object
                        .fields
                        .iter()
                        .map(|f| f.parsed_field.name.clone())
                        .collect()
                }),
            )
        })
        .collect()
}
