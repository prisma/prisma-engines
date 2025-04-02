use std::sync::Arc;

use codspeed_criterion_compat::{black_box, criterion_group, criterion_main, Criterion};
use query_core::BatchDocument;
use request_handlers::JsonSingleQuery;
use schema::QuerySchema;

const SCHEMA: &str = include_str!("../../schema/test-schemas/6573.prisma");

fn criterion_benchmark(c: &mut Criterion) {
    let validated_schema = psl::parse_schema(SCHEMA).unwrap();
    let query_schema = schema::build(Arc::new(validated_schema), true);
    let mut adapter = request_handlers::JsonProtocolAdapter::new(&query_schema);

    let queries = (0..20000)
        .map(|i| {
            format!(
                r#"{{
                "action": "findUnique",
                "modelName": "Account",
                "query": {{
                    "arguments": {{
                        "where": {{ "id": {i} }}
                    }},
                    "selection": {{
                        "$scalars": true
                    }}
                }}
        }}"#
            )
        })
        .map(|s| {
            let json_req: JsonSingleQuery = serde_json::from_str(s.as_str()).unwrap();

            adapter.convert_single(json_req).unwrap()
        });

    let batch_doc = BatchDocument::new(queries.collect(), None);

    c.bench_function("batching", |b| b.iter(|| batch(batch_doc.clone(), &query_schema)));
}

fn batch(batch_doc: BatchDocument, schema: &QuerySchema) {
    batch_doc.compact(schema);

    black_box(())
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
