use super::{inmemory_record_processor::InMemoryRecordProcessor, read};
use crate::{interpreter::InterpretationResult, query_ast::*};
use connector::{
    self, filter::Filter, ConnectionLike, QueryArguments, RelAggregationRow, RelAggregationSelection, ScalarCompare,
};
use prisma_models::{ManyRecords, ModelProjection, Record, RecordProjection, RelationFieldRef};
use prisma_value::PrismaValue;
use std::collections::HashMap;

#[tracing::instrument(skip(tx, query, parent_result, processor))]
pub async fn m2m(
    tx: &mut dyn ConnectionLike,
    query: &RelatedRecordsQuery,
    parent_result: Option<&ManyRecords>,
    processor: InMemoryRecordProcessor,
) -> InterpretationResult<(ManyRecords, Option<Vec<RelAggregationRow>>)> {
    let parent_field = &query.parent_field;
    let child_link_id = parent_field.related_field().linking_fields();

    // We know that in a m2m scenario, we always require the ID of the parent, nothing else.
    let parent_ids = match query.parent_projections {
        Some(ref links) => links.clone(),
        None => {
            let parent_model_id = query.parent_field.model().primary_identifier();
            parent_result
                .expect("[ID retrieval] No parent results present in the query graph for reading related records.")
                .projections(&parent_model_id)?
        }
    };

    if parent_ids.is_empty() {
        return Ok((ManyRecords::empty(&query.selected_fields), None));
    }

    let ids = tx.get_related_m2m_record_ids(&query.parent_field, &parent_ids).await?;
    if ids.is_empty() {
        return Ok((ManyRecords::empty(&query.selected_fields), None));
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
    // - there is no aggregation selection
    // - the selection set is the child_link_id
    let mut scalars =
        if query.args.do_nothing() && query.aggregation_selections.is_empty() && child_link_id == query.selected_fields
        {
            ManyRecords::from_projection(child_ids, &query.selected_fields).with_unique_records()
        } else {
            let mut args = query.args.clone();
            let filter = child_link_id.is_in(child_ids);

            args.filter = match args.filter {
                Some(existing_filter) => Some(Filter::and(vec![existing_filter, filter])),
                None => Some(filter),
            };

            tx.get_many_records(
                &query.parent_field.related_model(),
                args,
                &query.selected_fields,
                &query.aggregation_selections,
            )
            .await?
        };

    // Child id to parent ids
    let mut id_map: HashMap<RecordProjection, Vec<RecordProjection>> = HashMap::new();

    for (parent_id, child_id) in ids {
        match id_map.get_mut(&child_id) {
            Some(v) => v.push(parent_id),
            None => {
                id_map.insert(
                    child_id.ensure_type_coherence(),
                    vec![parent_id.ensure_type_coherence()],
                );
            }
        };
    }

    let fields = &scalars.field_names;
    let mut additional_records: Vec<(usize, Vec<Record>)> = vec![];

    for (index, record) in scalars.records.iter_mut().enumerate() {
        let record_id = record.projection(fields, &child_model_id)?;
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
    let (scalars, aggregation_rows) =
        read::extract_aggregation_rows_from_scalars(scalars, query.aggregation_selections.clone());

    Ok((scalars, aggregation_rows))
}

// [DTODO] This is implemented in an inefficient fashion, e.g. too much Arc cloning going on.
#[tracing::instrument(skip(
    tx,
    parent_field,
    parent_projections,
    parent_result,
    query_args,
    selected_fields,
    processor
))]
#[allow(clippy::too_many_arguments)]
pub async fn one2m(
    tx: &mut dyn ConnectionLike,
    parent_field: &RelationFieldRef,
    parent_projections: Option<Vec<RecordProjection>>,
    parent_result: Option<&ManyRecords>,
    query_args: QueryArguments,
    selected_fields: &ModelProjection,
    aggr_selections: Vec<RelAggregationSelection>,
    processor: InMemoryRecordProcessor,
) -> InterpretationResult<(ManyRecords, Option<Vec<RelAggregationRow>>)> {
    let parent_model_id = parent_field.model().primary_identifier();
    let parent_link_id = parent_field.linking_fields();
    let child_link_id = parent_field.related_field().linking_fields();

    // Primary ID to link ID
    let joined_projections = match parent_projections {
        Some(projections) => projections,
        None => {
            let extractor = parent_model_id.clone().merge(parent_link_id.clone());
            parent_result
                .expect("[ID retrieval] No parent results present in the query graph for reading related records.")
                .projections(&extractor)?
        }
    };

    // Maps the identifying link values to all primary IDs they are tied to.
    // Only the values are hashed for easier comparison.
    let mut link_mapping: HashMap<Vec<PrismaValue>, Vec<RecordProjection>> = HashMap::new();
    let idents = vec![parent_model_id, parent_link_id];
    let mut uniq_projections = Vec::new();

    for projection in joined_projections {
        let mut split = projection.split_into(&idents);
        let link_id = split.pop().unwrap();
        let id = split.pop().unwrap();
        let link_values: Vec<PrismaValue> = link_id.pairs.into_iter().map(|(_, v)| v).collect();

        match link_mapping.get_mut(&link_values) {
            Some(records) => records.push(id),
            None => {
                let mut ids = Vec::new();

                ids.push(id);
                uniq_projections.push(link_values.clone());
                link_mapping.insert(link_values, ids);
            }
        }
    }

    let uniq_projections: Vec<Vec<PrismaValue>> = uniq_projections
        .into_iter()
        .filter(|p| !p.iter().any(|v| v.is_null()))
        .collect();

    if uniq_projections.is_empty() {
        return Ok((ManyRecords::empty(selected_fields), None));
    }

    // a roundtrip can be avoided if:
    // - there is no additional filter
    // - there is no aggregation selection
    // - the selection set is the child_link_id
    let mut scalars = if query_args.do_nothing() && aggr_selections.is_empty() && &child_link_id == selected_fields {
        ManyRecords::from_projection(uniq_projections, selected_fields).with_unique_records()
    } else {
        let filter = child_link_id.is_in(uniq_projections);
        let mut args = query_args;

        args.filter = match args.filter {
            Some(existing_filter) => Some(Filter::and(vec![existing_filter, filter])),
            None => Some(filter),
        };
        tx.get_many_records(&parent_field.related_model(), args, selected_fields, &aggr_selections)
            .await?
    };

    // Inlining is done on the parent, this means that we need to write the primary parent ID
    // into the child records that we retrieved. The matching is done based on the parent link values.
    if parent_field.is_inlined_on_enclosing_model() {
        let mut additional_records = vec![];

        for mut record in scalars.records.iter_mut() {
            let child_link: RecordProjection = record.projection(&scalars.field_names, &child_link_id)?;
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
            let child_link: RecordProjection = record.projection(&scalars.field_names, &child_link_fields)?;
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

    let scalars = processor.apply(scalars);
    let (scalars, aggregation_rows) = read::extract_aggregation_rows_from_scalars(scalars, aggr_selections);

    Ok((scalars, aggregation_rows))
}
