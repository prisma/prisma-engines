//! Top level queries to satisfy the connector interface operations.
pub mod aggregate;
pub mod read;
pub mod write;

use crate::value::value_from_bson;
use mongodb::bson::Bson;
use mongodb::bson::Document;
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
