use query_core::{QueryDocument, QueryGraph, QueryGraphBuilder};
use request_handlers::{JsonBody, JsonSingleQuery, RequestBody};
use std::sync::Arc;

pub fn query_graph(schema: &str, query: JsonSingleQuery) -> QueryGraph {
    let schema = psl::validate_without_extensions(schema.into());
    assert!(!schema.diagnostics.has_errors(), "{:?}", schema.diagnostics);
    let query_schema = Arc::new(query_core::schema::build(Arc::new(schema), true));
    let QueryDocument::Single(query) = RequestBody::Json(JsonBody::Single(query))
        .into_doc(&query_schema)
        .unwrap()
    else {
        panic!("expected single query");
    };

    query_core::with_sync_unevaluated_request_context(|| QueryGraphBuilder::new(&query_schema).build(query).unwrap())
}
