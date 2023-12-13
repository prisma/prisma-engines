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

// Performs both metrics pushing and query logging. Query logging  might be disabled and thus
// the query_string might not need to be built, that's why rather than a query_string
// we receive a Builder, as it's not trivial to buid a query and we want to skip that when possible.
//
// As a reminder, the query string is not fed into mongo db directly, we built it for debugging
// purposes and it's only used when the query log is enabled. For querying mongo, we use the driver
// wire protocol to build queries from a graphql query rather than executing raw mongodb statements.
//
// As we don't have a mongodb query string, we need to create it from the driver object model, which
// we better skip it if we don't need it (i.e. when the query log is disabled.)
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

    // todo: emit tracing event with the appropriate log level only query_log is enabled. And fix log suscription
    let query_string = builder.build();
    debug!(target: "mongodb_query_connector::query", item_type = "query", is_query = true, query = %query_string, duration_ms = elapsed);

    res
}
