use codspeed_criterion_compat::{Criterion, black_box, criterion_group, criterion_main};
use query_core::{query_graph_builder::QueryGraphBuilder, *};
use request_handlers::JsonSingleQuery;
use schema::QuerySchema;
use std::sync::Arc;

const SCHEMA: &str = include_str!("../../schema/test-schemas/odoo.prisma");

const QUERIES: &[(&str, &str)] = &[
    (
        "small_read",
        r#"
        { "action": "findMany", "modelName": "website_redirect", "query": { "selection": { "id": true } } }
        "#,
    ),
    (
        "medium_read",
        r#"
        {
            "action": "findMany",
            "modelName": "website_redirect",
            "query": {
                "selection": {
                    "id": true,
                    "create_uid": true,
                    "create_date": true,
                    "type": true,
                    "active": true,
                    "sequence": true,
                    "url_from": true,
                    "url_to": true,
                    "website": {
                        "selection": {
                            "id": true,
                            "name": true,
                            "company_id": true,
                            "default_lang_id": true,
                            "default_lang_code": true,
                            "social_twitter": true
                        }
                    }
                }
            }
        }
        "#,
    ),
    ("large_read", include_str!("./large_read.json")),
    ("deep_read_query", include_str!("./deep_read_query.json")),
    ("mutation", include_str!("./mutation.json")),
];

fn criterion_benchmark(c: &mut Criterion) {
    let validated_schema = psl::parse_schema(SCHEMA).unwrap();
    let query_schema = schema::build(Arc::new(validated_schema), true);

    for (name, query) in QUERIES {
        c.bench_function(name, |b| b.iter(|| validate_and_plan(query, &query_schema)));
    }
}

fn validate_and_plan(query: &str, schema: &QuerySchema) {
    fn validate_and_plan_impl(query: &str, schema: &QuerySchema) {
        let json_request: JsonSingleQuery = serde_json::from_str(query).unwrap();
        let mut adapter = request_handlers::JsonProtocolAdapter::new(schema);
        let operation = adapter.convert_single(json_request).unwrap();
        QueryGraphBuilder::new(schema).build(operation).unwrap();
    }

    validate_and_plan_impl(query, schema);
    black_box(())
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
