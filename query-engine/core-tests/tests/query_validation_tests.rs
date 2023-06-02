use psl::ALL_PREVIEW_FEATURES;
use query_core::query_graph_builder::QueryGraphBuilder;
use request_handlers::JsonSingleQuery;
use serde_json::json;
use std::{io::Write as _, path::Path, sync::Arc};

const TESTS_ROOT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/query_validation_tests");

fn run_query_validation_test(query_file_path: &str) {
    let query_file_path = Path::new(TESTS_ROOT).join(query_file_path);
    let schema_path = query_file_path.with_file_name("schema.prisma");

    let query = std::fs::read_to_string(&query_file_path).unwrap();
    let schema = std::fs::read_to_string(schema_path).unwrap();

    let all_features = ALL_PREVIEW_FEATURES
        .active_features()
        .iter()
        .chain(ALL_PREVIEW_FEATURES.hidden_features())
        .collect();
    let parsed_schema = psl::parse_schema(schema).unwrap();
    let schema = Arc::new(schema::build_with_features(Arc::new(parsed_schema), all_features, true));

    let err_string = match validate(&query, &schema) {
        Ok(()) => panic!("these tests are only for errors, the query should fail to validate, but it did not"),
        Err(err) => {
            let value = json!(user_facing_errors::Error::from(err));
            serde_json::to_string_pretty(&value).unwrap()
        }
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

    if err_string.trim_end_matches('\n') == snapshot.trim_end_matches('\n') {
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

fn validate(query: &str, schema: &schema::QuerySchema) -> Result<(), request_handlers::HandlerError> {
    let json_request: JsonSingleQuery = serde_json::from_str(query).unwrap();
    let mut adapter = request_handlers::JsonProtocolAdapter::new(schema);
    let operation = adapter.convert_single(json_request)?;
    QueryGraphBuilder::new(schema)
        .build(operation)
        .map_err(query_core::CoreError::from)?;
    Ok(())
}
include!(concat!(env!("OUT_DIR"), "/query_validation_tests.rs"));
