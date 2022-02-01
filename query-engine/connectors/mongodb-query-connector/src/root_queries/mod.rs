//! Top level queries to satisfy the connector interface operations.
pub mod aggregate;
mod raw;
pub mod read;
pub mod write;

use crate::{output_meta::OutputMetaMapping, value::value_from_bson};
use mongodb::bson::Bson;
use mongodb::bson::Document;
use prisma_models::*;

/// Transforms a document to a `Record`, fields ordered as defined in `fields`.
fn document_to_record(mut doc: Document, fields: &[String], meta_mapping: &OutputMetaMapping) -> crate::Result<Record> {
    let mut values: Vec<PrismaValue> = Vec::with_capacity(fields.len());

    for field in fields {
        let bson = doc.remove(field).unwrap_or(Bson::Null);
        let mapping = meta_mapping.get(field).expect("Incorrect meta type mapping.");
        let val = value_from_bson(bson, mapping)?;

        values.push(val);
    }

    Ok(Record::new(values))
}
