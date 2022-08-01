//! Top level queries to satisfy the connector interface operations.
pub mod aggregate;
pub mod read;
pub mod write;

mod raw;
mod update;

use crate::{
    error::DecorateErrorWithFieldInformationExtension, output_meta::OutputMetaMapping, value::value_from_bson,
};
use futures::Future;
use metrics::{histogram, increment_counter};
use mongodb::bson::Bson;
use mongodb::bson::Document;
use prisma_models::*;
use std::time::Instant;

/// Transforms a document to a `Record`, fields ordered as defined in `fields`.
fn document_to_record(mut doc: Document, fields: &[String], meta_mapping: &OutputMetaMapping) -> crate::Result<Record> {
    let mut values: Vec<PrismaValue> = Vec::with_capacity(fields.len());

    for field in fields {
        let bson = doc.remove(field).unwrap_or(Bson::Null);
        let mapping = meta_mapping.get(field).expect("Incorrect meta type mapping.");
        let val = value_from_bson(bson, mapping).decorate_with_field_name(field)?;

        values.push(val);
    }

    Ok(Record::new(values))
}

/// We currently only allow a singular ID for Mongo, this helps extracting it.
fn pick_singular_id(model: &ModelRef) -> ScalarFieldRef {
    model
        .primary_identifier()
        .as_scalar_fields()
        .expect("ID contains non-scalars")
        .into_iter()
        .next()
        .unwrap()
}

pub(crate) async fn metrics<'a, F, T, U>(f: F) -> mongodb::error::Result<T>
where
    F: FnOnce() -> U + 'a,
    U: Future<Output = mongodb::error::Result<T>>,
{
    let start = Instant::now();
    let res = f().await;

    histogram!("prisma_datasource_queries_duration_histogram_ms", start.elapsed());
    increment_counter!("prisma_datasource_queries_total");

    res
}
