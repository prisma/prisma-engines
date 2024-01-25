use super::*;
use crate::{ArgumentListLookup, FieldPair, ParsedField, ReadQuery};
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

    let mut selected_fields = Vec::new();

    for pair in pairs {
        let field = parent.find_field(&pair.parsed_field.name);

        match (pair.parsed_field.clone(), field) {
            (pf, Some(Field::Relation(rf))) => {
                let fields = rf.scalar_fields().into_iter().map(SelectedField::from);

                selected_fields.extend(fields);

                if should_collect_relation_selection {
                    selected_fields.push(extract_relation_selection(pf, rf, query_schema)?);
                }
            }

            (_, Some(Field::Scalar(sf))) => {
                selected_fields.push(sf.into());
            }

            (pf, Some(Field::Composite(cf))) => {
                selected_fields.push(extract_composite_selection(pf, cf, query_schema)?);
            }

            (pf, None) if pf.name == UNDERSCORE_COUNT => match parent {
                ParentContainer::Model(ref model) => {
                    selected_fields.extend(extract_relation_count_selections(pf, model)?);
                }
                ParentContainer::CompositeType(_) => {
                    unreachable!("Unexpected relation aggregation selection inside a composite type query")
                }
            },

            (pf, None) => unreachable!(
                "Field '{}' does not exist on enclosing type and is not a known virtual field",
                pf.name
            ),
        }
    }

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

fn extract_relation_count_selections(
    pf: ParsedField<'_>,
    model: &Model,
) -> QueryGraphBuilderResult<Vec<SelectedField>> {
    let object = pf
        .nested_fields
        .expect("Invalid query shape: relation aggregation virtual field selected without relations to aggregate.");

    object
        .fields
        .into_iter()
        .map(|mut nested_pair| -> QueryGraphBuilderResult<_> {
            let rf = model
                .fields()
                .find_from_relation_fields(&nested_pair.parsed_field.name)
                .expect("Selected relation in relation aggregation virtual field must exist on the model");

            let filter = nested_pair
                .parsed_field
                .arguments
                .lookup(args::WHERE)
                .map(|where_arg| extract_filter(where_arg.value.try_into()?, rf.related_model()))
                .transpose()?;

            Ok(SelectedField::Virtual(VirtualSelection::RelationCount(rf, filter)))
        })
        .collect()
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

pub(crate) fn get_relation_load_strategy(
    requested_strategy: Option<RelationLoadStrategy>,
    cursor: Option<&SelectionResult>,
    distinct: Option<&FieldSelection>,
    nested_queries: &[ReadQuery],
    query_schema: &QuerySchema,
) -> RelationLoadStrategy {
    if query_schema.has_feature(PreviewFeature::RelationJoins)
        && query_schema.has_capability(ConnectorCapability::LateralJoin)
        && cursor.is_none()
        && distinct.is_none()
        && !nested_queries.iter().any(|q| match q {
            ReadQuery::RelatedRecordsQuery(q) => q.has_cursor() || q.has_distinct(),
            _ => false,
        })
        && requested_strategy != Some(RelationLoadStrategy::Query)
    {
        RelationLoadStrategy::Join
    } else {
        RelationLoadStrategy::Query
    }
}
