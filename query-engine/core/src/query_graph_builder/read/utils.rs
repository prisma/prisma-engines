use super::*;
use crate::{ArgumentListLookup, FieldPair, ParsedField, ReadQuery};
use connector::RelAggregationSelection;
use psl::{datamodel_connector::ConnectorCapability, PreviewFeature};
use query_structure::{prelude::*, RelationLoadStrategy};
use schema::{
    constants::{aggregations::*, args},
    QuerySchema,
};

pub fn collect_selection_order(from: &[FieldPair<'_>]) -> Vec<String> {
    from.iter()
        .map(|pair| {
            pair.parsed_field
                .alias
                .clone()
                .unwrap_or_else(|| pair.parsed_field.name.clone())
        })
        .collect()
}

/// Creates a `FieldSelection` from a query selection.
/// Automatically adds model IDs to the selected fields as well.
pub fn collect_selected_fields(
    from_pairs: &[FieldPair<'_>],
    distinct: Option<FieldSelection>,
    model: &Model,
    query_schema: &QuerySchema,
) -> QueryGraphBuilderResult<FieldSelection> {
    let model_id = model.primary_identifier();
    let selected_fields = pairs_to_selections(model, from_pairs, query_schema)?;

    let selection = FieldSelection::new(selected_fields);
    let selection = model_id.merge(selection);

    // Distinct fields are always selected because we are processing them in-memory
    if let Some(distinct) = distinct {
        Ok(selection.merge(distinct))
    } else {
        Ok(selection)
    }
}

/// Creates a `FieldSelection` from a query selection, which contains only scalar fields.
/// Automatically adds model IDs to the selected fields as well.
pub fn collect_selected_scalars(from_pairs: &[FieldPair<'_>], model: &Model) -> FieldSelection {
    let model_id = model.primary_identifier();
    let selected_fields = pairs_to_scalar_selections(model, from_pairs);
    let selection = FieldSelection::new(selected_fields);

    model_id.merge(selection)
}

fn pairs_to_scalar_selections<T>(parent: T, pairs: &[FieldPair<'_>]) -> Vec<SelectedField>
where
    T: Into<ParentContainer>,
{
    let parent: ParentContainer = parent.into();

    pairs
        .iter()
        .filter_map(|pair| parent.find_field(&pair.parsed_field.name))
        .filter_map(|field| field.into_scalar())
        .map(SelectedField::from)
        .collect()
}

fn pairs_to_selections<T>(
    parent: T,
    pairs: &[FieldPair<'_>],
    query_schema: &QuerySchema,
) -> QueryGraphBuilderResult<Vec<SelectedField>>
where
    T: Into<ParentContainer>,
{
    let should_collect_relation_selection = query_schema.has_capability(ConnectorCapability::LateralJoin)
        && query_schema.has_feature(PreviewFeature::RelationJoins);

    let parent = parent.into();

    let selected_fields = pairs
        .iter()
        .filter_map(|pair| {
            parent
                .find_field(&pair.parsed_field.name)
                .map(|field| (pair.parsed_field.clone(), field))
        })
        .flat_map(|field| match field {
            (pf, Field::Relation(rf)) => {
                let mut fields: Vec<QueryGraphBuilderResult<SelectedField>> = rf
                    .scalar_fields()
                    .into_iter()
                    .map(SelectedField::from)
                    .map(Ok)
                    .collect();

                if should_collect_relation_selection {
                    fields.push(extract_relation_selection(pf, rf, query_schema));
                }

                fields
            }
            (_, Field::Scalar(sf)) => vec![Ok(sf.into())],
            (pf, Field::Composite(cf)) => vec![extract_composite_selection(pf, cf, query_schema)],
        })
        .collect::<Result<Vec<_>, _>>()?;

    Ok(selected_fields)
}

fn extract_composite_selection(
    pf: ParsedField<'_>,
    cf: CompositeFieldRef,
    query_schema: &QuerySchema,
) -> QueryGraphBuilderResult<SelectedField> {
    let object = pf
        .nested_fields
        .expect("Invalid composite query shape: Composite field selected without sub-selection.");

    let typ = cf.typ();

    Ok(SelectedField::Composite(CompositeSelection {
        field: cf,
        selections: pairs_to_selections(typ, &object.fields, query_schema)?,
    }))
}

fn extract_relation_selection(
    pf: ParsedField<'_>,
    rf: RelationFieldRef,
    query_schema: &QuerySchema,
) -> QueryGraphBuilderResult<SelectedField> {
    let object = pf
        .nested_fields
        .expect("Invalid relation query shape: Relation field selected without sub-selection.");

    let related_model = rf.related_model();

    Ok(SelectedField::Relation(RelationSelection {
        field: rf,
        args: extract_query_args(pf.arguments, &related_model)?,
        result_fields: collect_selection_order(&object.fields),
        selections: pairs_to_selections(related_model, &object.fields, query_schema)?,
    }))
}

pub(crate) fn collect_nested_queries(
    from: Vec<FieldPair<'_>>,
    model: &Model,
    query_schema: &QuerySchema,
) -> QueryGraphBuilderResult<Vec<ReadQuery>> {
    from.into_iter()
        .filter_map(|pair| {
            if is_aggr_selection(&pair) {
                return None;
            }

            let model_field = model.fields().find_from_all(&pair.parsed_field.name).unwrap();

            match model_field {
                Field::Scalar(_) => None,
                Field::Composite(_) => None,
                Field::Relation(ref rf) => {
                    let model = rf.related_model();
                    let parent = rf.clone();

                    Some(related::find_related(pair.parsed_field, parent, model, query_schema))
                }
            }
        })
        .collect::<QueryGraphBuilderResult<Vec<ReadQuery>>>()
}

/// Performs a lookahead based on the nested queries and merges fields required
/// to resolve the nested queries.
/// A lookback on the parent is also performed to ensure that fields required for
/// resolving the parent relation are present.
pub(crate) fn merge_relation_selections(
    selected_fields: FieldSelection,
    parent_relation: Option<RelationFieldRef>,
    nested_queries: &[ReadQuery],
) -> FieldSelection {
    // Context: We are on the child model when calling this function.
    let selected_fields = if let Some(rf) = parent_relation {
        let field = rf.related_field();
        selected_fields.merge(field.linking_fields())
    } else {
        selected_fields
    };

    let nested: Vec<_> = nested_queries
        .iter()
        .map(|nested_query| {
            if let ReadQuery::RelatedRecordsQuery(ref rq) = nested_query {
                rq.parent_field.linking_fields()
            } else {
                unreachable!()
            }
        })
        .collect();

    selected_fields.merge(FieldSelection::union(nested))
}

/// Ensures that if a cursor is provided, its fields are also selected.
/// Necessary for post-processing of unstable orderings with cursor operations.
pub fn merge_cursor_fields(selected_fields: FieldSelection, cursor: &Option<SelectionResult>) -> FieldSelection {
    match cursor {
        Some(cursor) => selected_fields.merge(cursor.into()),
        None => selected_fields,
    }
}

pub fn collect_relation_aggr_selections(
    from: Vec<FieldPair<'_>>,
    model: &Model,
) -> QueryGraphBuilderResult<Vec<RelAggregationSelection>> {
    let mut selections = vec![];

    for pair in from {
        match pair.parsed_field.name.as_str() {
            UNDERSCORE_COUNT => {
                let nested_fields = pair.parsed_field.nested_fields.unwrap();

                for mut nested_pair in nested_fields.fields {
                    let rf = model
                        .fields()
                        .find_from_relation_fields(&nested_pair.parsed_field.name)
                        .unwrap();
                    let filter = match nested_pair.parsed_field.arguments.lookup(args::WHERE) {
                        Some(where_arg) => Some(extract_filter(where_arg.value.try_into()?, rf.related_model())?),
                        _ => None,
                    };

                    selections.push(RelAggregationSelection::Count(rf, filter));
                }
            }
            field_name => panic!("Unknown field name \"{field_name}\" for a relation aggregation"),
        }
    }

    Ok(selections)
}

pub(crate) fn get_relation_load_strategy(
    requested_strategy: Option<RelationLoadStrategy>,
    cursor: Option<&SelectionResult>,
    distinct: Option<&FieldSelection>,
    nested_queries: &[ReadQuery],
    aggregation_selections: &[RelAggregationSelection],
    query_schema: &QuerySchema,
) -> RelationLoadStrategy {
    if query_schema.has_feature(PreviewFeature::RelationJoins)
        && query_schema.has_capability(ConnectorCapability::LateralJoin)
        && cursor.is_none()
        && distinct.is_none()
        && aggregation_selections.is_empty()
        && !nested_queries.iter().any(|q| match q {
            ReadQuery::RelatedRecordsQuery(q) => q.has_cursor() || q.has_distinct() || q.has_aggregation_selections(),
            _ => false,
        })
        // && supports_m2m_ordering
        && requested_strategy != Some(RelationLoadStrategy::Query)
    {
        RelationLoadStrategy::Join
    } else {
        RelationLoadStrategy::Query
    }
}
