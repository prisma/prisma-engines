#![cfg(feature = "kingbase-oracle")]

use quaint::prelude::{ConnectionInfo, ExternalConnectionInfo, SqlFamily};
use query_core::{QueryDocument, protocol::EngineProtocol};
use request_handlers::RequestBody;
use std::{fs, sync::Arc};

const UNSUPPORTED_FIXTURES: &[&str] = &[
    // Kingbase Oracle does not expose Prisma scalar lists.
    "data-types.json",
    "update-push.json",
];

const LATERAL_JOIN_FIXTURES: &[&str] = &["query-m2o-lateral.json", "query-o2m-lateral.json"];

#[test]
fn compiles_all_supported_standard_query_fixtures_with_the_oracle_visitor() {
    let datamodel = include_str!("data/schema.prisma")
        .replace("provider = \"postgresql\"", "provider = \"kingbase-oracle\"")
        // `data-types.json` is excluded above, but the schema must still be valid while the
        // rest of the standard fixture matrix is compiled.
        .replace("intArray  Int[]", "intArray  Int");
    let schema = psl::validate_without_extensions(datamodel.into());

    assert!(!schema.diagnostics.has_errors(), "{:?}", schema.diagnostics);

    let query_schema = Arc::new(schema::build(Arc::new(schema), true));
    let connection_info = ConnectionInfo::External(ExternalConnectionInfo::new(
        SqlFamily::KingbaseOracle,
        Some("public".to_owned()),
        None,
        false,
    ));

    for fixture in fs::read_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/data")).unwrap() {
        let fixture = fixture.unwrap();
        let file_name = fixture.file_name();
        let file_name = file_name.to_string_lossy();

        if !file_name.ends_with(".json") || UNSUPPORTED_FIXTURES.contains(&file_name.as_ref()) {
            continue;
        }

        let request: serde_json::Value = serde_json::from_str(&fs::read_to_string(fixture.path()).unwrap()).unwrap();
        let request = serde_json::to_string(&request).unwrap();
        let request = RequestBody::try_from_str(&request, EngineProtocol::Json)
            .unwrap_or_else(|error| panic!("{file_name}: failed to parse fixture: {error}"));
        let QueryDocument::Single(operation) = request
            .into_doc(&query_schema)
            .unwrap_or_else(|error| panic!("{file_name}: failed to build query document: {error}"))
        else {
            panic!("{file_name}: expected a single query");
        };

        let plan = query_compiler::compile(&query_schema, operation, &connection_info)
            .unwrap_or_else(|error| panic!("{file_name}: failed to compile with the Oracle visitor: {error}"));

        if LATERAL_JOIN_FIXTURES.contains(&file_name.as_ref()) {
            let rendered = plan.pretty_print(false, 80).unwrap();
            assert!(
                rendered.contains("LEFT JOIN LATERAL"),
                "{file_name}: expected a lateral relation join:\n{rendered}"
            );
        }
    }
}

#[test]
fn compiles_verified_oracle_filter_variants() {
    let schema = psl::validate_without_extensions(
        r#"
            datasource db {
                provider = "kingbase-oracle"
            }

            model TestModel {
                id        Int    @id
                json      Json
                json2     Json?
                stringRef String?
            }
        "#
        .into(),
    );

    assert!(!schema.diagnostics.has_errors(), "{:?}", schema.diagnostics);

    let query_schema = Arc::new(schema::build(Arc::new(schema), true));
    let connection_info = ConnectionInfo::External(ExternalConnectionInfo::new(
        SqlFamily::KingbaseOracle,
        Some("public".to_owned()),
        None,
        false,
    ));

    let requests: &[(&str, &[&str], Option<&str>)] = &[
        (
            r#"{"modelName":"TestModel","action":"findMany","query":{"arguments":{"where":{"json":{"path":["profile","name"],"equals":"\\\"Ada\\\""}}},"selection":{"$scalars":true}}}"#,
            &["jsonb_extract_path(", "VARIADIC CAST("],
            Some(r#"{"profile","name"}"#),
        ),
        (
            r#"{"modelName":"TestModel","action":"findMany","query":{"arguments":{"where":{"json":{"path":["score"],"gt":"5"}}},"selection":{"$scalars":true}}}"#,
            &["jsonb_extract_path(", "VARIADIC CAST("],
            Some(r#"{"score"}"#),
        ),
        (
            r#"{"modelName":"TestModel","action":"findMany","query":{"arguments":{"where":{"json":{"path":["profile","name"],"string_contains":"d"}}},"selection":{"$scalars":true}}}"#,
            &["jsonb_extract_path_text(", "jsonb_extract_path("],
            Some(r#"{"profile","name"}"#),
        ),
        (
            r#"{"modelName":"TestModel","action":"findMany","query":{"arguments":{"where":{"json":{"path":["tags"],"array_contains":"[\\\"prisma\\\"]"}}},"selection":{"$scalars":true}}}"#,
            &["jsonb_extract_path(", "VARIADIC CAST("],
            Some(r#"{"tags"}"#),
        ),
        (
            r#"{"modelName":"TestModel","action":"findMany","query":{"arguments":{"where":{"json":{"path":["tags"],"array_starts_with":"\\\"prisma\\\"","array_ends_with":"\\\"orm\\\""}}},"selection":{"$scalars":true}}}"#,
            &["jsonb_extract_path(", "JSON_QUERY", "$[0]", "$[last]"],
            Some(r#"{"tags"}"#),
        ),
        (
            r#"{"modelName":"TestModel","action":"findMany","query":{"arguments":{"where":{"json":{"path":["score"],"gt":{"$type":"FieldRef","value":{"_ref":"json2","_container":"TestModel"}}}}},"selection":{"$scalars":true}}}"#,
            &["jsonb_extract_path(", "VARIADIC CAST("],
            Some(r#"{"score"}"#),
        ),
        (
            r#"{"modelName":"TestModel","action":"findMany","query":{"arguments":{"where":{"stringRef":{"contains":"lov","mode":"insensitive"}}},"selection":{"$scalars":true}}}"#,
            &["ILIKE"],
            None,
        ),
        (
            r#"{"modelName":"TestModel","action":"findMany","query":{"arguments":{"where":{"json":{"path":["profile","name"],"string_contains":"lov","mode":"insensitive"}}},"selection":{"$scalars":true}}}"#,
            &["LOWER(jsonb_extract_path_text(", "jsonb_extract_path("],
            Some(r#"{"profile","name"}"#),
        ),
        (
            r#"{"modelName":"TestModel","action":"findMany","query":{"arguments":{"where":{"stringRef":{"search":"prisma & compiler"}}},"selection":{"$scalars":true}}}"#,
            &["to_tsvector"],
            None,
        ),
        (
            r#"{"modelName":"TestModel","action":"findMany","query":{"arguments":{"orderBy":{"_relevance":{"fields":["stringRef"],"search":"prisma & compiler","sort":"desc"}}},"selection":{"$scalars":true}}}"#,
            &["ORDER BY ts_rank", " DESC"],
            None,
        ),
        (
            r#"{"modelName":"TestModel","action":"findMany","query":{"arguments":{"where":{"json":{"path":["map","0"],"equals":"\"zero\""}}},"selection":{"$scalars":true}}}"#,
            &["jsonb_extract_path(", "VARIADIC CAST("],
            Some(r#"{"map","0"}"#),
        ),
    ];

    query_core::with_sync_unevaluated_request_context(|| {
        for &(request, expected_sql_fragments, expected_path) in requests {
            let request = RequestBody::try_from_str(request, EngineProtocol::Json).unwrap();
            let QueryDocument::Single(operation) = request.into_doc(&query_schema).unwrap() else {
                panic!("expected a single query");
            };

            let plan = query_compiler::compile(&query_schema, operation, &connection_info).unwrap();
            let rendered = plan.pretty_print(false, 80).unwrap();

            if let Some(expected_path) = expected_path {
                fn contains_string(value: &serde_json::Value, expected: &str) -> bool {
                    match value {
                        serde_json::Value::String(value) => value == expected,
                        serde_json::Value::Array(values) => values.iter().any(|value| contains_string(value, expected)),
                        serde_json::Value::Object(values) => {
                            values.values().any(|value| contains_string(value, expected))
                        }
                        _ => false,
                    }
                }
                assert!(
                    contains_string(&serde_json::to_value(&plan).unwrap(), expected_path),
                    "missing bound JSON path {expected_path}:\n{rendered}"
                );
            }

            for expected_sql in expected_sql_fragments {
                assert!(rendered.contains(expected_sql), "expected {expected_sql}:\n{rendered}");
            }
        }
    });
}

#[test]
fn relation_join_maps_nested_blob_fields_as_hex() {
    let schema = psl::validate_without_extensions(
        r#"
            generator client {
                provider        = "prisma-client"
                previewFeatures = ["relationJoins"]
            }

            datasource db {
                provider = "kingbase-oracle"
            }

            model Parent {
                id       Int     @id
                children Child[]
            }

            model Child {
                id       Int    @id
                payload  Bytes  @db.Blob
                parentId Int
                parent   Parent @relation(fields: [parentId], references: [id])
            }
        "#
        .into(),
    );

    assert!(!schema.diagnostics.has_errors(), "{:?}", schema.diagnostics);

    let query_schema = Arc::new(schema::build(Arc::new(schema), true));
    let connection_info = ConnectionInfo::External(ExternalConnectionInfo::new(
        SqlFamily::KingbaseOracle,
        Some("public".to_owned()),
        None,
        false,
    ));
    let request = RequestBody::try_from_str(
        r#"{"modelName":"Parent","action":"findMany","query":{"arguments":{"relationLoadStrategy":"join"},"selection":{"id":true,"children":{"arguments":{},"selection":{"payload":true}}}}}"#,
        EngineProtocol::Json,
    )
    .unwrap();
    let QueryDocument::Single(operation) = request.into_doc(&query_schema).unwrap() else {
        panic!("expected a single query");
    };

    let plan = query_compiler::compile(&query_schema, operation, &connection_info).unwrap();
    let serialized_plan = serde_json::to_value(plan).unwrap();

    assert_eq!(
        serialized_plan.pointer("/args/structure/fields/children/fields/payload/fieldType/encoding"),
        Some(&serde_json::Value::String("hex".to_owned()))
    );
}
