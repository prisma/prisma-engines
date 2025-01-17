use super::*;
use crate::{
    error::DecorateErrorWithFieldInformationExtension, output_meta, query_builder::MongoReadQueryBuilder,
    query_strings::Find, vacuum_cursor, IntoBson,
};
use mongodb::{bson::doc, options::FindOptions, ClientSession, Database};
use query_structure::*;
use std::future::IntoFuture;

/// Finds a single record. Joins are not required at the moment because the selector is always a unique one.
pub async fn get_single_record(
    database: &Database,
    session: &mut ClientSession,
    model: &Model,
    filter: &Filter,
    selected_fields: &FieldSelection,
) -> crate::Result<Option<SingleRecord>> {
    let coll = database.collection(model.db_name());

    let meta_mapping = output_meta::from_selected_fields(selected_fields);
    let query_arguments: QueryArguments = (model.clone(), filter.clone()).into();
    let query = MongoReadQueryBuilder::from_args(query_arguments)?
        .with_model_projection(selected_fields.clone())?
        .with_virtual_fields(selected_fields.virtuals())?
        .build()?;

    let docs = query.execute(coll, session).await?;

    if docs.is_empty() {
        Ok(None)
    } else {
        let field_names: Vec<_> = selected_fields.db_names().collect();
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
pub async fn get_many_records(
    database: &Database,
    session: &mut ClientSession,
    model: &Model,
    query_arguments: QueryArguments,
    selected_fields: &FieldSelection,
) -> crate::Result<ManyRecords> {
    let coll = database.collection(model.db_name());

    let reverse_order = query_arguments.take.map(|t| t < 0).unwrap_or(false);
    let field_names: Vec<_> = selected_fields.db_names().collect();

    let meta_mapping = output_meta::from_selected_fields(selected_fields);
    let mut records = ManyRecords::new(field_names.clone());

    if let Some(0) = query_arguments.take {
        return Ok(records);
    };

    let query = MongoReadQueryBuilder::from_args(query_arguments)?
        .with_model_projection(selected_fields.clone())?
        .with_virtual_fields(selected_fields.virtuals())?
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

pub async fn get_related_m2m_record_ids(
    database: &Database,
    session: &mut ClientSession,
    from_field: &RelationFieldRef,
    from_record_ids: &[SelectionResult],
) -> crate::Result<Vec<(SelectionResult, SelectionResult)>> {
    if from_record_ids.is_empty() {
        return Ok(vec![]);
    }

    let model = from_field.model();
    let coll = database.collection(model.db_name());
    let id_field = pick_singular_id(&model);
    let ids = from_record_ids
        .iter()
        .map(|p| {
            (&id_field, p.values().next().unwrap())
                .into_bson()
                .decorate_with_scalar_field_info(&id_field)
        })
        .collect::<crate::Result<Vec<_>>>()?;

    // Scalar field name where the relation ids list is on `model`.
    let id_holder_field = from_field.scalar_fields().into_iter().next().unwrap();
    let relation_ids_field_name = id_holder_field.name().to_owned();

    let filter = doc! { id_field.db_name(): { "$in": ids } };
    let projection = doc! { id_field.db_name(): 1, relation_ids_field_name: 1 };

    let query_string_builder = Find::new(&filter, &projection, coll.name());
    let find_options = FindOptions::builder().projection(projection.clone()).build();

    let cursor = observing(&query_string_builder, || {
        coll.find(filter.clone())
            .with_options(find_options)
            .session(&mut *session)
            .into_future()
    })
    .await?;
    let docs = vacuum_cursor(cursor, session).await?;
    let parent_id_meta = output_meta::from_scalar_field(&id_field);
    let related_ids_holder_meta = output_meta::from_scalar_field(&id_holder_field);
    let child_id_field = pick_singular_id(&from_field.related_model());

    let mut id_pairs = vec![];
    for mut doc in docs {
        let id_value = doc.remove(id_field.db_name()).unwrap();
        let parent_id = value_from_bson(id_value, &parent_id_meta)?;

        let related_id_array = doc
            .remove(id_holder_field.name())
            .unwrap_or_else(|| Bson::Array(vec![]));

        let child_ids: Vec<PrismaValue> = match value_from_bson(related_id_array, &related_ids_holder_meta)? {
            PrismaValue::List(vals) => vals,
            val => vec![val],
        };

        let parent_projection = SelectionResult::from((id_field.clone(), parent_id));

        for child_id in child_ids {
            let child_projection = SelectionResult::from((child_id_field.clone(), child_id));
            id_pairs.push((parent_projection.clone(), child_projection));
        }
    }

    Ok(id_pairs)
}
