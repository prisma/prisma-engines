use crate::{value::value_from_bson, BsonTransform, IntoBson};
use connector_interface::Filter;
use futures::stream::StreamExt;
use mongodb::{bson::Bson, options::FindOptions};
use mongodb::{bson::Document, Cursor, Database};
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
        let field_names = selected_fields
            .scalar_fields()
            .map(|sf| sf.db_name().to_owned())
            .collect::<Vec<_>>();

        let doc = docs.into_iter().next().unwrap();
        let record = document_to_record(doc, &field_names)?;

        Ok(Some(SingleRecord { record, field_names }))
    }
}

/// Transforms a document to a `Record`, fields ordered as defined in `fields`.
fn document_to_record(mut doc: Document, fields: &[String]) -> crate::Result<Record> {
    let mut values: Vec<PrismaValue> = Vec::with_capacity(fields.len());

    for field in fields {
        let bson = doc.remove(field).unwrap_or(Bson::Null);
        let val = value_from_bson(bson)?;

        values.push(val);
    }

    Ok(Record::new(values))
}

async fn vacuum_cursor(mut cursor: Cursor) -> crate::Result<Vec<Document>> {
    let mut docs = vec![];

    while let Some(result) = cursor.next().await {
        match result {
            Ok(document) => docs.push(document),
            Err(e) => return Err(e.into()),
        }
    }

    Ok(docs)
}
