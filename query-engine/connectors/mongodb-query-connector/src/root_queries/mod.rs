//! Top level queries to satisfy the connector interface operations.
pub mod aggregate;
pub mod read;
pub mod write;

mod raw;
mod update;

use crate::query_strings::QueryString;
use crate::{
    error::DecorateErrorWithFieldInformationExtension, output_meta::OutputMetaMapping, value::value_from_bson,
};
use futures::Future;
use mongodb::bson::Bson;
use mongodb::bson::Document;
use query_engine_metrics::{
    histogram, increment_counter, metrics, PRISMA_DATASOURCE_QUERIES_DURATION_HISTOGRAM_MS,
    PRISMA_DATASOURCE_QUERIES_TOTAL,
};
use query_structure::*;
use std::time::Instant;
use tracing::debug;

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
fn pick_singular_id(model: &Model) -> ScalarFieldRef {
    model
        .primary_identifier()
        .as_scalar_fields()
        .expect("ID contains non-scalars")
        .into_iter()
        .next()
        .unwrap()
}

/// Logs the query and updates metrics for an operation performed by a passed function.
///
/// NOTE:
/// 1. `dyn QueryString` is used instead of a `String` to skip expensive query serialization when
///    query logs are disabled. This, however, is not currently implemented.
/// 2. Query strings logged are for debugging purposes only. The actual queries constructed by
///    MongoDB driver might look slightly different.
pub(crate) async fn observing<'a, 'b, F, T, U>(builder: &'b dyn QueryString, f: F) -> mongodb::error::Result<T>
where
    F: FnOnce() -> U + 'a,
    U: Future<Output = mongodb::error::Result<T>>,
{
    let start = Instant::now();
    let res = f().await;
    let elapsed = start.elapsed().as_millis() as f64;

    histogram!(PRISMA_DATASOURCE_QUERIES_DURATION_HISTOGRAM_MS, elapsed);
    increment_counter!(PRISMA_DATASOURCE_QUERIES_TOTAL);

    // TODO: emit tracing event only when "debug" level query logs are enabled.
    // TODO prisma/team-orm#136: fix log subscription.
    let query_string = builder.build();
    // NOTE: `params` is a part of the interface for query logs.
    let params: Vec<i32> = vec![];
    debug!(target: "mongodb_query_connector::query", item_type = "query", is_query = true, query = %query_string, params = ?params, duration_ms = elapsed);

    res
}
