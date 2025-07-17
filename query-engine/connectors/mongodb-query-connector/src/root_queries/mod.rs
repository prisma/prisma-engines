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
use bson::Bson;
use bson::Document;
use futures::Future;
use prisma_metrics::{
    PRISMA_DATASOURCE_QUERIES_DURATION_HISTOGRAM_MS, PRISMA_DATASOURCE_QUERIES_TOTAL, counter, histogram,
};
use query_structure::*;
use std::sync::Arc;
use std::time::Instant;
use tracing::{debug, info_span};
use tracing_futures::Instrument;

const DB_SYSTEM_NAME: &str = "mongodb";

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
    // TODO: build the string lazily in the Display impl so it doesn't have to be built if neither
    // logs nor traces are enabled. This is tricky because whatever we store in the span has to be
    // 'static, and all `QueryString` implementations aren't, so this requires some refactoring.
    let query_string: Arc<str> = builder.build().into();

    let span = info_span!(
        "prisma:engine:db_query",
        user_facing = true,
        "db.system" = DB_SYSTEM_NAME,
        "db.query.text" = %Arc::clone(&query_string),
        "db.operation.name" = builder.query_type(),
        "otel.kind" = "client"
    );

    if let Some(coll) = builder.collection() {
        span.record("db.collection.name", coll);
    }

    let start = Instant::now();
    let res = f().instrument(span).await;
    let elapsed = start.elapsed().as_millis() as f64;

    histogram!(PRISMA_DATASOURCE_QUERIES_DURATION_HISTOGRAM_MS).record(elapsed);
    counter!(PRISMA_DATASOURCE_QUERIES_TOTAL).increment(1);

    // TODO prisma/team-orm#136: fix log subscription.
    // NOTE: `params` is a part of the interface for query logs.
    debug!(target: "mongodb_query_connector::query", item_type = "query", is_query = true, query = %query_string, params = %"[]", duration_ms = elapsed);

    res
}
