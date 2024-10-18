use super::inmemory_record_processor::InMemoryRecordProcessor;
use crate::{interpreter::InterpretationResult, query_ast::*};
use connector::ConnectionLike;
use query_structure::*;
use std::collections::HashMap;
use telemetry::helpers::TraceParent;

pub(crate) async fn m2m(
    tx: &mut dyn ConnectionLike,
    query: &mut RelatedRecordsQuery,
    parent_result: Option<&ManyRecords>,
    traceparent: Option<TraceParent>,
) -> InterpretationResult<ManyRecords> {
    let processor = InMemoryRecordProcessor::new_from_query_args(&mut query.args);

    let parent_field = &query.parent_field;
    let child_link_id = parent_field.related_field().linking_fields();

    // We know that in a m2m scenario, we always require the ID of the parent, nothing else.
    let parent_ids = match query.parent_results {
        Some(ref links) => links.clone(),
        None => {
            let parent_model_id = query.parent_field.model().primary_identifier();
            parent_result
                .expect("[ID retrieval] No parent results present in the query graph for reading related records.")
                .extract_selection_results_from_db_name(&parent_model_id)?
        }
    };

    if parent_ids.is_empty() {
        return Ok(ManyRecords::empty(&query.selected_fields));
    }

    let ids = tx
        .get_related_m2m_record_ids(&query.parent_field, &parent_ids, traceparent)
        .await?;
    if ids.is_empty() {
        return Ok(ManyRecords::empty(&query.selected_fields));
    }

    let child_model_id = query.parent_field.related_model().primary_identifier();

    let child_ids: Vec<Vec<PrismaValue>> = ids
        .iter()
        .map(|ri| {
            let proj = child_model_id.assimilate(ri.1.clone());
            proj.map(|ri| ri.values().collect::<Vec<_>>())
        })
        .collect::<std::result::Result<Vec<_>, _>>()?;

    // a roundtrip can be avoided if:
    // - there is no additional filter
    // - there is no virtual fields selection (relation aggregation)
    // - the selection set is the child_link_id
    let mut scalars = if query.args.do_nothing()
        && !query.selected_fields.has_virtual_fields()
        && child_link_id == query.selected_fields
    {
        ManyRecords::from((child_ids, &query.selected_fields)).with_unique_records()
    } else {
        let mut args = query.args.clone();
        let filter = child_link_id.is_in(ConditionListValue::list(child_ids));

        args.filter = match args.filter {
            Some(existing_filter) => Some(Filter::and(vec![existing_filter, filter])),
            None => Some(filter),
        };

        tx.get_many_records(
            &query.parent_field.related_model(),
            args,
            &query.selected_fields,
            RelationLoadStrategy::Query,
            traceparent,
        )
        .await?
    };

    // Child id to parent ids
    let mut id_map: HashMap<SelectionResult, Vec<SelectionResult>> = HashMap::new();

    for (parent_id, child_id) in ids {
        let parent_id = parent_id.coerce_values()?;
        let child_id = child_id.coerce_values()?;

        match id_map.get_mut(&child_id) {
            Some(v) => v.push(parent_id),
            None => {
                id_map.insert(child_id, vec![parent_id]);
            }
        };
    }

    let fields = &scalars.field_names;
    let mut additional_records: Vec<(usize, Vec<Record>)> = vec![];

    for (index, record) in scalars.records.iter_mut().enumerate() {
        let record_id = record.extract_selection_result_from_db_name(fields, &child_model_id)?;
        let mut parent_ids = id_map.remove(&record_id).expect("1");
        let first = parent_ids.pop().expect("2");

        record.parent_id = Some(first);

        let mut more_records = vec![];

        for parent_id in parent_ids {
            let mut record = record.clone();

            record.parent_id = Some(parent_id);
            more_records.push(record);
        }

        if !more_records.is_empty() {
            additional_records.push((index + 1, more_records));
        }
    }

    // Start to insert in the back to keep other indices valid.
    additional_records.reverse();

    for (index, records) in additional_records {
        for (offset, record) in records.into_iter().enumerate() {
            scalars.records.insert(index + offset, record);
        }
    }

    let scalars = processor.apply(scalars);

    Ok(scalars)
}

// [DTODO] This is implemented in an inefficient fashion, e.g. too much Arc cloning going on.
#[allow(clippy::too_many_arguments)]
pub async fn one2m(
    tx: &mut dyn ConnectionLike,
    parent_field: &RelationFieldRef,
    parent_selections: Option<Vec<SelectionResult>>,
    parent_result: Option<&ManyRecords>,
    mut query_args: QueryArguments,
    selected_fields: &FieldSelection,
    traceparent: Option<TraceParent>,
) -> InterpretationResult<ManyRecords> {
    let parent_model_id = parent_field.model().primary_identifier();
    let parent_link_id = parent_field.linking_fields();
    let child_link_id = parent_field.related_field().linking_fields();

    // Primary ID to link ID
    let joined_results = match parent_selections {
        Some(selections) => selections,
        None => {
            let extractor = parent_model_id.clone().merge(parent_link_id.clone());
            parent_result
                .expect("[ID retrieval] No parent results present in the query graph for reading related records.")
                .extract_selection_results_from_db_name(&extractor)?
        }
    };

    // Maps the identifying link values to all primary IDs they are tied to.
    // Only the values are hashed for easier comparison.
    let mut link_mapping: HashMap<Vec<PrismaValue>, Vec<SelectionResult>> = HashMap::new();
    let link_idents = vec![parent_model_id, parent_link_id];
    let mut uniq_selections = Vec::new();

    for result in joined_results {
        let mut split = result.split_into(&link_idents);
        let link_id = split.pop().unwrap();
        let id = split.pop().unwrap();
        let link_values: Vec<PrismaValue> = link_id.pairs.into_iter().map(|(_, v)| v).collect();

        match link_mapping.get_mut(&link_values) {
            Some(records) => records.push(id),
            None => {
                let ids = vec![id];
                uniq_selections.push(link_values.clone());
                link_mapping.insert(link_values, ids);
            }
        }
    }

    let uniq_selections: Vec<Vec<PrismaValue>> = uniq_selections
        .into_iter()
        .filter(|p| !p.iter().any(|v| v.is_null()))
        .collect();

    if uniq_selections.is_empty() {
        return Ok(ManyRecords::empty(selected_fields));
    }

    // If we're fetching related records from a single parent, then we can apply normal pagination instead of in-memory processing.
    // However, we can't just apply a LIMIT/OFFSET for multiple parents as we need N related records PER parent.
    // We could use ROW_NUMBER() but it requires further refactoring so we're still using in-memory processing for now.
    let processor = if uniq_selections.len() == 1 && !query_args.requires_inmemory_processing() {
        None
    } else {
        Some(InMemoryRecordProcessor::new_from_query_args(&mut query_args))
    };

    let mut scalars = {
        let filter = child_link_id.is_in(ConditionListValue::list(uniq_selections));
        let mut args = query_args;

        args.filter = match args.filter {
            Some(existing_filter) => Some(Filter::and(vec![existing_filter, filter])),
            None => Some(filter),
        };

        tx.get_many_records(
            &parent_field.related_model(),
            args,
            selected_fields,
            RelationLoadStrategy::Query,
            traceparent,
        )
        .await?
    };

    // Inlining is done on the parent, this means that we need to write the primary parent ID
    // into the child records that we retrieved. The matching is done based on the parent link values.
    if parent_field.is_inlined_on_enclosing_model() {
        let mut additional_records = vec![];

        for record in scalars.records.iter_mut() {
            let child_link: SelectionResult =
                record.extract_selection_result_from_db_name(&scalars.field_names, &child_link_id)?;
            let child_link_values: Vec<PrismaValue> = child_link.pairs.into_iter().map(|(_, v)| v).collect();

            if let Some(parent_ids) = link_mapping.get_mut(&child_link_values) {
                parent_ids.reverse();

                let parent_id = parent_ids.pop().unwrap();
                record.parent_id = Some(parent_id);

                for parent_id in parent_ids {
                    let mut record = record.clone();

                    record.parent_id = Some((*parent_id).clone());
                    additional_records.push(record);
                }
            }
        }

        scalars.records.extend(additional_records);
    } else if parent_field.related_field().is_inlined_on_enclosing_model() {
        // Parent to map is inlined on the child records
        let child_link_fields = parent_field.related_field().linking_fields();

        for record in scalars.records.iter_mut() {
            let child_link: SelectionResult =
                record.extract_selection_result_from_db_name(&scalars.field_names, &child_link_fields)?;
            let child_link_values: Vec<PrismaValue> = child_link.pairs.into_iter().map(|(_, v)| v).collect();

            if let Some(parent_ids) = link_mapping.get(&child_link_values) {
                let parent_id = parent_ids.last().unwrap();
                record.parent_id = Some(parent_id.clone());
            }
        }
    } else {
        panic!(
            "parent result: {:?}, relation: {:?}",
            &parent_result,
            &parent_field.relation()
        );
    }

    let scalars = if let Some(processor) = processor {
        processor.apply(scalars)
    } else {
        scalars
    };

    Ok(scalars)
}
