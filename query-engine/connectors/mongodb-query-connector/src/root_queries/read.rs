use super::*;
use crate::{output_meta, query_builder::MongoReadQueryBuilder, vacuum_cursor, IntoBson};
use connector_interface::{Filter, QueryArguments, RelAggregationSelection};
use mongodb::{bson::doc, options::FindOptions, ClientSession, Database};
use prisma_models::*;

/// Finds a single record. Joins are not required at the moment because the selector is always a unique one.
pub async fn get_single_record<'conn>(
    database: &Database,
    session: &mut ClientSession,
    model: &ModelRef,
    filter: &Filter,
    selected_fields: &ModelProjection,
    aggregation_selections: &[RelAggregationSelection],
) -> crate::Result<Option<SingleRecord>> {
    let coll = database.collection(model.db_name());
    let meta_mapping = output_meta::from_selected_fields(selected_fields, aggregation_selections);
    let query_arguments: QueryArguments = (model.clone(), filter.clone()).into();
    let query = MongoReadQueryBuilder::from_args(query_arguments)?
        .with_model_projection(selected_fields.clone())?
        .with_aggregation_selections(aggregation_selections)?
        .build()?;

    let docs = query.execute(coll, session).await?;

    if docs.is_empty() {
        Ok(None)
    } else {
        let field_names: Vec<_> = selected_fields
            .db_names()
            .chain(aggregation_selections.iter().map(|aggr_sel| aggr_sel.db_alias()))
            .collect();
        let doc = docs.into_iter().next().unwrap();
        let record = document_to_record(doc, &field_names, &meta_mapping)?;

        Ok(Some(SingleRecord { record, field_names }))
    }
}

// Checklist:
// - [x] OrderBy scalar.
// - [ ] OrderBy relation.
// - [x] Skip, take
// - [x] Cursor
// - [x] Distinct select (inherently given from core).
// - [x] Relation aggregation count
pub async fn get_many_records<'conn>(
    database: &Database,
    session: &mut ClientSession,
    model: &ModelRef,
    query_arguments: QueryArguments,
    selected_fields: &ModelProjection,
    aggregation_selections: &[RelAggregationSelection],
) -> crate::Result<ManyRecords> {
    let coll = database.collection(model.db_name());
    let reverse_order = query_arguments.take.map(|t| t < 0).unwrap_or(false);
    let field_names: Vec<_> = selected_fields
        .db_names()
        .chain(aggregation_selections.iter().map(|aggr_sel| aggr_sel.db_alias()))
        .collect();

    let meta_mapping = output_meta::from_selected_fields(selected_fields, aggregation_selections);
    let mut records = ManyRecords::new(field_names.clone());

    if let Some(0) = query_arguments.take {
        return Ok(records);
    };

    let query = MongoReadQueryBuilder::from_args(query_arguments)?
        .with_model_projection(selected_fields.clone())?
        .with_aggregation_selections(aggregation_selections)?
        .build()?;

    let docs = query.execute(coll, session).await?;
    for doc in docs {
        let record = document_to_record(doc, &field_names, &meta_mapping)?;
        records.push(record)
    }

    if reverse_order {
        records.reverse();
    }

    Ok(records)
}

pub async fn get_related_m2m_record_ids<'conn>(
    database: &Database,
    session: &mut ClientSession,
    from_field: &RelationFieldRef,
    from_record_ids: &[RecordProjection],
) -> crate::Result<Vec<(RecordProjection, RecordProjection)>> {
    if from_record_ids.is_empty() {
        return Ok(vec![]);
    }

    let model = from_field.model();
    let coll = database.collection(model.db_name());

    let id_field = model.primary_identifier().scalar_fields().next().unwrap();
    let ids = from_record_ids
        .iter()
        .map(|p| (&id_field, p.values().next().unwrap()).into_bson())
        .collect::<crate::Result<Vec<_>>>()?;

    let filter = doc! { id_field.db_name(): { "$in": ids } };

    // Scalar field name where the relation ids list is on `model`.
    let relation_ids_field_name = from_field.relation_info.fields.get(0).unwrap();

    let find_options = FindOptions::builder()
        .projection(doc! { id_field.db_name(): 1, relation_ids_field_name: 1 })
        .build();

    let cursor = coll.find_with_session(filter, Some(find_options), session).await?;

    let docs = vacuum_cursor(cursor, session).await?;

    let parent_id_meta = output_meta::from_field(&id_field);
    let id_holder_field = model.fields().find_from_scalar(relation_ids_field_name).unwrap();
    let related_ids_holder_meta = output_meta::from_field(&id_holder_field);

    let child_id_field = from_field
        .related_model()
        .primary_identifier()
        .scalar_fields()
        .next()
        .unwrap();

    let mut id_pairs = vec![];
    for mut doc in docs {
        let id_value = doc.remove(id_field.db_name()).unwrap();
        let parent_id = value_from_bson(id_value, &parent_id_meta)?;

        let related_id_array = doc
            .remove(relation_ids_field_name)
            .unwrap_or_else(|| Bson::Array(vec![]));

        let child_ids: Vec<PrismaValue> = match value_from_bson(related_id_array, &related_ids_holder_meta)? {
            PrismaValue::List(vals) => vals,
            val => vec![val],
        };

        let parent_projection = RecordProjection::from((id_field.clone(), parent_id));

        for child_id in child_ids {
            let child_projection = RecordProjection::from((child_id_field.clone(), child_id));
            id_pairs.push((parent_projection.clone(), child_projection));
        }
    }

    Ok(id_pairs)
}
