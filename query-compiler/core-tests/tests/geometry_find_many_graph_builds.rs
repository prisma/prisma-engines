use std::sync::Arc;

use query_core::{QueryDocument, QueryGraphBuilder};
use request_handlers::{JsonBody, JsonSingleQuery, RequestBody};

#[test]
fn geometry_find_many_builds_query_graph() {
    let schema_string = r#"
        datasource db {
            provider = "postgresql"
        }

        generator client {
            provider        = "prisma-client"
            previewFeatures = ["relationJoins"]
        }

        model Location {
            id       Int                    @id @default(autoincrement())
            position Geometry(Point, 4326)
        }
    "#;

    let schema = psl::validate_without_extensions(schema_string.into());
    assert!(!schema.diagnostics.has_errors(), "{:?}", schema.diagnostics);

    let schema = Arc::new(schema);
    let query_schema = Arc::new(query_core::schema::build(schema, true));

    let query_json = r#"{
        "modelName": "Location",
        "action": "findMany",
        "query": {
            "arguments": {},
            "selection": {
                "id": true,
                "position": true
            }
        }
    }"#;

    let query: JsonSingleQuery = serde_json::from_str(query_json).unwrap();
    let request = RequestBody::Json(JsonBody::Single(query));
    let doc = request.into_doc(&query_schema).unwrap();

    let QueryDocument::Single(query) = doc else {
        panic!("expected single query");
    };

    QueryGraphBuilder::new(&query_schema)
        .build(query)
        .expect("findMany with geometry fields should compile to a query graph");
}
