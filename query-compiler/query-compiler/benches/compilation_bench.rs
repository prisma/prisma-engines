//! End-to-end compilation benchmarks for the query compiler.
//!
//! This benchmark uses the same schema and query fixtures as the unit tests
//! in `tests/data/` to measure full compilation pipeline performance.
//!
//! New benchmarks are automatically discovered when new query JSON files
//! are added to the `tests/data/` directory.
//!
//! ## Running benchmarks
//!
//! ```bash
//! cargo bench -p query-compiler
//! ```
//!
//! ## Filtering benchmarks
//!
//! ```bash
//! cargo bench -p query-compiler -- "create"
//! cargo bench -p query-compiler -- "query-m2o"
//! ```

use codspeed_criterion_compat::{Criterion, black_box, criterion_group, criterion_main};
use itertools::Itertools;
use quaint::prelude::{ConnectionInfo, ExternalConnectionInfo, SqlFamily};
use query_compiler::compile;
use request_handlers::{JsonBody, JsonSingleQuery, RequestBody};
use schema::QuerySchema;
use std::sync::Arc;
use std::{fs, path::PathBuf};

fn get_test_data_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/data")
}

struct BenchContext {
    query_schema: Arc<QuerySchema>,
    connection_info: ConnectionInfo,
}

impl BenchContext {
    fn new() -> Self {
        let data_dir = get_test_data_dir();
        let schema_path = data_dir.join("schema.prisma");
        let schema_str = fs::read_to_string(&schema_path)
            .unwrap_or_else(|e| panic!("Failed to read {}: {}", schema_path.display(), e));

        let validated_schema = psl::parse_schema_without_extensions(&schema_str).unwrap();
        let query_schema = Arc::new(schema::build(Arc::new(validated_schema), true));
        let connection_info = ConnectionInfo::External(ExternalConnectionInfo::new(
            SqlFamily::Postgres,
            Some("public".to_string()),
            None,
            true,
        ));
        Self {
            query_schema,
            connection_info,
        }
    }

    fn compile(&self, query_json: &str) {
        let query: JsonSingleQuery = serde_json::from_str(query_json).unwrap();
        let request = RequestBody::Json(JsonBody::Single(query));
        let doc = request.into_doc(&self.query_schema).unwrap();

        let query_core::QueryDocument::Single(operation) = doc else {
            panic!("expected single query");
        };

        let mut expr = compile(&self.query_schema, operation, &self.connection_info).unwrap();
        expr.simplify();
        black_box(expr);
    }
}

fn discover_query_files() -> impl IntoIterator<Item = (String, String)> {
    let data_dir = get_test_data_dir();

    fs::read_dir(&data_dir)
        .expect("failed to read data directory")
        .map(|entry| entry.expect("failed to read directory entry").path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "json"))
        .sorted()
        .map(|path| {
            let name = path
                .file_stem()
                .expect("file should have a stem")
                .to_string_lossy()
                .into_owned();

            let content =
                fs::read_to_string(&path).unwrap_or_else(|e| panic!("failed to read {}: {}", path.display(), e));

            (name, content)
        })
}

fn compilation_benchmarks(c: &mut Criterion) {
    let ctx = BenchContext::new();
    let queries = discover_query_files();

    let mut group = c.benchmark_group("compile");

    for (name, query_json) in queries {
        group.bench_function(name, |b| {
            b.iter(|| ctx.compile(&query_json));
        });
    }

    group.finish();
}

criterion_group!(benches, compilation_benchmarks);
criterion_main!(benches);
