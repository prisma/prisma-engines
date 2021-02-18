pub mod aggregate;
pub mod read;
pub mod write;

use crate::value::value_from_bson;
use futures::stream::StreamExt;
use mongodb::bson::Bson;
use mongodb::{bson::Document, Cursor};
use prisma_models::*;

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

/// Consumes a cursor stream until exhausted.
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
