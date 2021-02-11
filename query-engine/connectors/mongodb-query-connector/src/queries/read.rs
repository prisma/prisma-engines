use super::*;
use crate::{BsonTransform, IntoBson};
use connector_interface::{Filter, QueryArguments};
use mongodb::options::FindOptions;
use mongodb::Database;
use prisma_models::*;

pub async fn get_single_record(
    database: &Database,
    model: &ModelRef,
    filter: &Filter,
    selected_fields: &ModelProjection,
) -> crate::Result<Option<SingleRecord>> {
    let coll = database.collection(model.db_name());

    // Todo look at interfaces (req clones).
    let filter = filter.clone().into_bson()?.into_document()?;
    let find_options = FindOptions::builder()
        .projection(selected_fields.clone().into_bson()?.into_document()?)
        .build();

    let cursor = coll.find(Some(filter), Some(find_options)).await?;
    let docs = vacuum_cursor(cursor).await?;

    if docs.len() == 0 {
        Ok(None)
    } else {
        let field_names: Vec<_> = selected_fields.db_names().collect();
        let doc = docs.into_iter().next().unwrap();
        let record = document_to_record(doc, &field_names)?;

        Ok(Some(SingleRecord { record, field_names }))
    }
}

// Checklist:
// - [ ] OrderBy scalar.
// - [ ] OrderBy relation.
// - [ ] Cursors (skip, take, cursor).
// - [ ] Distinct select.
pub async fn get_many_records(
    database: &Database,
    model: &ModelRef,
    mut query_arguments: QueryArguments,
    selected_fields: &ModelProjection,
) -> crate::Result<ManyRecords> {
    let coll = database.collection(model.db_name());
    // let reversed = query_arguments.take.map(|t| t < 0).unwrap_or(false);
    let field_names: Vec<_> = selected_fields.db_names().collect();
    let mut records = ManyRecords::new(field_names.clone());

    if let Some(0) = query_arguments.take {
        return Ok(records);
    };

    let filter = match query_arguments.filter {
        Some(filter) => Some(filter.into_bson()?.into_document()?),
        None => None,
    };

    let find_options = FindOptions::builder()
        .projection(selected_fields.clone().into_bson()?.into_document()?)
        .build();

    let cursor = coll.find(filter, Some(find_options)).await?;
    let docs = vacuum_cursor(cursor).await?;

    for doc in docs {
        let record = document_to_record(doc, &field_names)?;
        records.push(record)
    }

    Ok(records)
}
