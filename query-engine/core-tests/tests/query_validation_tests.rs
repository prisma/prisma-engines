use query_core::query_graph_builder::QueryGraphBuilder;
use request_handlers::JsonSingleQuery;
use serde_json::json;
use std::{io::Write as _, path::Path, sync::Arc};

const TESTS_ROOT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/query_validation_tests");

fn run_query_validation_test(query_file_path: &str) {
    let query_file_path = Path::new(TESTS_ROOT).join(query_file_path);
    let schema_path = query_file_path.with_file_name("schema.prisma");

    let query = std::fs::read_to_string(&query_file_path).unwrap();
    let schema = std::fs::read_to_string(&schema_path).unwrap();

    let parsed_schema = psl::parse_schema(schema).unwrap();
    let prisma_models_schema = prisma_models::convert(Arc::new(parsed_schema));
    let schema = Arc::new(schema_builder::build(prisma_models_schema, true));

    let err_string = match validate(&query, schema) {
        Ok(()) => panic!("these tests are only for errors, the query should fail to validate, but it did not"),
        Err(err) => json!(user_facing_errors::Error::from(err)).to_string(),
    };

    let snapshot_path = query_file_path.parent().unwrap().with_file_name(
        query_file_path
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .replace(".query.json", ".expected.json"),
    );

    let snapshot = std::fs::read_to_string(&snapshot_path).unwrap_or_default();

    if err_string == snapshot {
        return; // test passed
    }

    if std::env::var("UPDATE_EXPECT").as_deref() == Ok("1") {
        let mut snapshot_file = std::fs::File::create(&snapshot_path).unwrap();
        snapshot_file.write_all(err_string.as_bytes()).unwrap();
        return;
    }

    panic_with_diff(&snapshot, &err_string)
}

fn panic_with_diff(expected: &str, found: &str) {
    let chunks = dissimilar::diff(expected, found);
    let diff = format_chunks(chunks);
    panic!(
        r#"
Snapshot comparison failed. Run the test again with UPDATE_EXPECT=1 in the environment to update the snapshot.

===== EXPECTED ====
{expected}
====== FOUND ======
{found}
======= DIFF ======
{diff}
        "#
    );
}

fn format_chunks(chunks: Vec<dissimilar::Chunk<'_>>) -> String {
    let mut buf = String::new();
    for chunk in chunks {
        let formatted = match chunk {
            dissimilar::Chunk::Equal(text) => text.into(),
            dissimilar::Chunk::Delete(text) => format!("\x1b[41m{text}\x1b[0m"),
            dissimilar::Chunk::Insert(text) => format!("\x1b[42m{text}\x1b[0m"),
        };
        buf.push_str(&formatted);
    }
    buf
}

fn validate(query: &str, schema: schema::QuerySchemaRef) -> Result<(), request_handlers::HandlerError> {
    let json_request: JsonSingleQuery = serde_json::from_str(&query).unwrap();
    let operation = request_handlers::JsonProtocolAdapter::convert_single(json_request, &schema)?;
    QueryGraphBuilder::new(schema)
        .build(operation)
        .map_err(query_core::CoreError::from)?;
    Ok(())
}

// #[test]
// fn postgres_basic_create_with_non_existent_field() {
//     run_query_validation_test("postgres_basic/selection_is_empty.query.json");
// }

include!(concat!(env!("OUT_DIR"), "/query_validation_tests.rs"));
