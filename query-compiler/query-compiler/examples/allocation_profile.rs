//! Allocation profile for query compilation phases.
//!
//! This example counts heap allocation calls and requested bytes for the native
//! query compiler path. It is intentionally separate from `profile_query` so
//! the timing/flamegraph example does not pay counting-allocator overhead.
//!
//! ```bash
//! cargo run -p query-compiler --example allocation_profile --release
//! ALLOC_PROFILE_QUERIES=query-m2o,nested-pagination-query \
//!   cargo run -p query-compiler --example allocation_profile --release
//! ALLOC_PROFILE_BUCKETS=1 ALLOC_PROFILE_QUERIES=create-nested-create \
//!   cargo run -p query-compiler --example allocation_profile --release
//! ```

use quaint::prelude::{ConnectionInfo, ExternalConnectionInfo, SqlFamily};
use quaint::visitor;
use query_compiler::{Expression, compile, translate};
use query_core::{Operation, QueryDocument, QueryGraph, QueryGraphBuilder};
use request_handlers::{JsonBody, JsonSingleQuery, RequestBody};
use schema::QuerySchemaRef;
use sql_query_builder::{Context, SqlQueryBuilder};
use std::alloc::{GlobalAlloc, Layout, System};
use std::fs;
use std::hint::black_box;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

#[global_allocator]
static GLOBAL: CountingAllocator = CountingAllocator;

static ALLOCATIONS: AtomicU64 = AtomicU64::new(0);
static DEALLOCATIONS: AtomicU64 = AtomicU64::new(0);
static BYTES_ALLOCATED: AtomicU64 = AtomicU64::new(0);
static BYTES_DEALLOCATED: AtomicU64 = AtomicU64::new(0);
const ALLOC_BUCKET_LIMITS: [usize; 21] = [
    8,
    16,
    24,
    32,
    48,
    64,
    96,
    128,
    192,
    256,
    384,
    512,
    768,
    1024,
    1536,
    2048,
    4096,
    8192,
    16_384,
    65_536,
    usize::MAX,
];
const ALLOC_BUCKET_COUNT: usize = ALLOC_BUCKET_LIMITS.len();
static BUCKET_ALLOCATIONS: [AtomicU64; ALLOC_BUCKET_COUNT] = [const { AtomicU64::new(0) }; ALLOC_BUCKET_COUNT];
static BUCKET_BYTES_ALLOCATED: [AtomicU64; ALLOC_BUCKET_COUNT] = [const { AtomicU64::new(0) }; ALLOC_BUCKET_COUNT];

struct CountingAllocator;

unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let ptr = unsafe { System.alloc(layout) };
        if !ptr.is_null() {
            ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
            BYTES_ALLOCATED.fetch_add(layout.size() as u64, Ordering::Relaxed);
            record_bucket_allocation(layout.size());
        }
        ptr
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        if !ptr.is_null() {
            DEALLOCATIONS.fetch_add(1, Ordering::Relaxed);
            BYTES_DEALLOCATED.fetch_add(layout.size() as u64, Ordering::Relaxed);
        }
        unsafe { System.dealloc(ptr, layout) };
    }

    unsafe fn realloc(&self, ptr: *mut u8, old_layout: Layout, new_size: usize) -> *mut u8 {
        let new_ptr = unsafe { System.realloc(ptr, old_layout, new_size) };
        if !new_ptr.is_null() {
            DEALLOCATIONS.fetch_add(1, Ordering::Relaxed);
            BYTES_DEALLOCATED.fetch_add(old_layout.size() as u64, Ordering::Relaxed);
            ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
            BYTES_ALLOCATED.fetch_add(new_size as u64, Ordering::Relaxed);
            record_bucket_allocation(new_size);
        }
        new_ptr
    }
}

fn record_bucket_allocation(size: usize) {
    let bucket = allocation_bucket(size);
    BUCKET_ALLOCATIONS[bucket].fetch_add(1, Ordering::Relaxed);
    BUCKET_BYTES_ALLOCATED[bucket].fetch_add(size as u64, Ordering::Relaxed);
}

fn allocation_bucket(size: usize) -> usize {
    ALLOC_BUCKET_LIMITS
        .iter()
        .position(|limit| size <= *limit)
        .expect("last allocation bucket must be usize::MAX")
}

#[derive(Clone, Copy, Default)]
struct AllocSnapshot {
    allocations: u64,
    deallocations: u64,
    bytes_allocated: u64,
    bytes_deallocated: u64,
    bucket_allocations: [u64; ALLOC_BUCKET_COUNT],
    bucket_bytes_allocated: [u64; ALLOC_BUCKET_COUNT],
}

impl AllocSnapshot {
    fn reset() {
        ALLOCATIONS.store(0, Ordering::Relaxed);
        DEALLOCATIONS.store(0, Ordering::Relaxed);
        BYTES_ALLOCATED.store(0, Ordering::Relaxed);
        BYTES_DEALLOCATED.store(0, Ordering::Relaxed);
        for bucket in 0..ALLOC_BUCKET_COUNT {
            BUCKET_ALLOCATIONS[bucket].store(0, Ordering::Relaxed);
            BUCKET_BYTES_ALLOCATED[bucket].store(0, Ordering::Relaxed);
        }
    }

    fn current() -> Self {
        let mut bucket_allocations = [0; ALLOC_BUCKET_COUNT];
        let mut bucket_bytes_allocated = [0; ALLOC_BUCKET_COUNT];
        for bucket in 0..ALLOC_BUCKET_COUNT {
            bucket_allocations[bucket] = BUCKET_ALLOCATIONS[bucket].load(Ordering::Relaxed);
            bucket_bytes_allocated[bucket] = BUCKET_BYTES_ALLOCATED[bucket].load(Ordering::Relaxed);
        }

        Self {
            allocations: ALLOCATIONS.load(Ordering::Relaxed),
            deallocations: DEALLOCATIONS.load(Ordering::Relaxed),
            bytes_allocated: BYTES_ALLOCATED.load(Ordering::Relaxed),
            bytes_deallocated: BYTES_DEALLOCATED.load(Ordering::Relaxed),
            bucket_allocations,
            bucket_bytes_allocated,
        }
    }
}

#[derive(Default)]
struct AllocTotals {
    samples: u64,
    allocations: u64,
    deallocations: u64,
    bytes_allocated: u64,
    bytes_deallocated: u64,
    bucket_allocations: [u64; ALLOC_BUCKET_COUNT],
    bucket_bytes_allocated: [u64; ALLOC_BUCKET_COUNT],
}

impl AllocTotals {
    fn push(&mut self, snapshot: AllocSnapshot) {
        self.samples += 1;
        self.allocations += snapshot.allocations;
        self.deallocations += snapshot.deallocations;
        self.bytes_allocated += snapshot.bytes_allocated;
        self.bytes_deallocated += snapshot.bytes_deallocated;
        for bucket in 0..ALLOC_BUCKET_COUNT {
            self.bucket_allocations[bucket] += snapshot.bucket_allocations[bucket];
            self.bucket_bytes_allocated[bucket] += snapshot.bucket_bytes_allocated[bucket];
        }
    }

    fn print(&self, phase: &str) {
        let samples = self.samples as f64;
        let allocations = self.allocations as f64 / samples;
        let deallocations = self.deallocations as f64 / samples;
        let bytes_allocated = self.bytes_allocated as f64 / samples;
        let bytes_deallocated = self.bytes_deallocated as f64 / samples;
        let net_bytes = bytes_allocated - bytes_deallocated;

        println!(
            "  {phase:<16} allocs/op={allocations:>8.1} deallocs/op={deallocations:>8.1} allocated/op={} deallocated/op={} net/op={}",
            format_bytes(bytes_allocated),
            format_bytes(bytes_deallocated),
            format_bytes(net_bytes),
        );
    }

    fn print_buckets(&self) {
        let samples = self.samples as f64;
        let mut buckets = (0..ALLOC_BUCKET_COUNT)
            .map(|bucket| {
                (
                    bucket,
                    self.bucket_allocations[bucket] as f64 / samples,
                    self.bucket_bytes_allocated[bucket] as f64 / samples,
                )
            })
            .filter(|(_, allocations, _)| *allocations > 0.0)
            .collect::<Vec<_>>();

        buckets.sort_by(|left, right| right.2.partial_cmp(&left.2).unwrap_or(std::cmp::Ordering::Equal));

        for (bucket, allocations, bytes_allocated) in buckets.into_iter().take(6) {
            println!(
                "    {:<14} allocs/op={allocations:>7.1} allocated/op={}",
                bucket_label(bucket),
                format_bytes(bytes_allocated),
            );
        }
    }
}

fn bucket_label(bucket: usize) -> String {
    let upper = ALLOC_BUCKET_LIMITS[bucket];
    if bucket == 0 {
        return format!("<= {}", format_bytes(upper as f64));
    }

    if upper == usize::MAX {
        return format!("> {}", format_bytes(ALLOC_BUCKET_LIMITS[bucket - 1] as f64));
    }

    format!(
        "{}..{}",
        format_bytes((ALLOC_BUCKET_LIMITS[bucket - 1] + 1) as f64),
        format_bytes(upper as f64),
    )
}

struct ProfileContext {
    query_schema: QuerySchemaRef,
    connection_info: ConnectionInfo,
}

impl ProfileContext {
    fn new() -> Self {
        let data_dir = get_test_data_dir();
        let schema_path = data_dir.join("schema.prisma");
        let schema = fs::read_to_string(&schema_path)
            .unwrap_or_else(|err| panic!("failed to read {}: {err}", schema_path.display()));
        let validated_schema = psl::parse_schema_without_extensions(&schema).unwrap();
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

    fn parse_query(&self, query_json: &str) -> JsonSingleQuery {
        serde_json::from_str(query_json).unwrap()
    }

    fn query_to_operation(&self, query: JsonSingleQuery) -> Operation {
        let request = RequestBody::Json(JsonBody::Single(query));
        let doc = request.into_doc(&self.query_schema).unwrap();

        let QueryDocument::Single(operation) = doc else {
            panic!("expected single query");
        };

        operation
    }

    fn compile_operation(&self, operation: Operation) -> Expression {
        compile(&self.query_schema, operation, &self.connection_info).unwrap()
    }

    fn build_graph(&self, operation: Operation) -> QueryGraph {
        QueryGraphBuilder::new(&self.query_schema).build(operation).unwrap()
    }

    fn translate_graph(&self, graph: QueryGraph) -> Expression {
        let ctx = Context::new(&self.connection_info, None);
        translate(graph, &SqlQueryBuilder::<visitor::Postgres<'_>>::new(ctx)).unwrap()
    }

    fn compile_query(&self, query_json: &str) -> Expression {
        let query = self.parse_query(query_json);
        let operation = self.query_to_operation(query);
        self.compile_operation(operation)
    }
}

fn get_test_data_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/data")
}

fn default_query_names() -> Vec<String> {
    [
        "query-m2o",
        "query-many-m2m",
        "nested-pagination-query",
        "filter-contains-param",
        "create-nested-create",
    ]
    .into_iter()
    .map(ToOwned::to_owned)
    .collect()
}

fn query_names() -> Vec<String> {
    std::env::var("ALLOC_PROFILE_QUERIES")
        .ok()
        .map(|value| {
            value
                .split(',')
                .map(str::trim)
                .filter(|name| !name.is_empty())
                .map(ToOwned::to_owned)
                .collect()
        })
        .unwrap_or_else(default_query_names)
}

fn load_query(data_dir: &Path, name: &str) -> String {
    let path = data_dir.join(format!("{name}.json"));
    fs::read_to_string(&path).unwrap_or_else(|err| panic!("failed to read {}: {err}", path.display()))
}

fn measure_phase(iterations: usize, mut phase: impl FnMut() -> AllocSnapshot) -> AllocTotals {
    let mut totals = AllocTotals::default();
    for _ in 0..iterations {
        totals.push(phase());
    }
    totals
}

fn print_phase(totals: AllocTotals, phase: &str, print_buckets: bool) {
    totals.print(phase);
    if print_buckets {
        totals.print_buckets();
    }
}

fn measure_parse(ctx: &ProfileContext, query_json: &str) -> AllocSnapshot {
    AllocSnapshot::reset();
    let query = ctx.parse_query(query_json);
    black_box(&query);
    AllocSnapshot::current()
}

fn measure_into_doc(ctx: &ProfileContext, query_json: &str) -> AllocSnapshot {
    let query = ctx.parse_query(query_json);
    AllocSnapshot::reset();
    let operation = ctx.query_to_operation(query);
    black_box(&operation);
    AllocSnapshot::current()
}

fn measure_compile(ctx: &ProfileContext, query_json: &str) -> AllocSnapshot {
    let operation = ctx.query_to_operation(ctx.parse_query(query_json));
    AllocSnapshot::reset();
    let expression = ctx.compile_operation(operation);
    black_box(&expression);
    AllocSnapshot::current()
}

fn measure_graph_build(ctx: &ProfileContext, query_json: &str) -> AllocSnapshot {
    let operation = ctx.query_to_operation(ctx.parse_query(query_json));
    AllocSnapshot::reset();
    let graph = ctx.build_graph(operation);
    black_box(&graph);
    AllocSnapshot::current()
}

fn measure_translate(ctx: &ProfileContext, query_json: &str) -> AllocSnapshot {
    let operation = ctx.query_to_operation(ctx.parse_query(query_json));
    let graph = ctx.build_graph(operation);
    AllocSnapshot::reset();
    let expression = ctx.translate_graph(graph);
    black_box(&expression);
    AllocSnapshot::current()
}

fn measure_full(ctx: &ProfileContext, query_json: &str) -> AllocSnapshot {
    AllocSnapshot::reset();
    let expression = ctx.compile_query(query_json);
    black_box(&expression);
    AllocSnapshot::current()
}

fn measure_serialize(ctx: &ProfileContext, query_json: &str) -> AllocSnapshot {
    let expression = ctx.compile_query(query_json);
    AllocSnapshot::reset();
    let serialized = serde_json::to_string(&expression).unwrap();
    black_box(&serialized);
    AllocSnapshot::current()
}

fn format_bytes(bytes: f64) -> String {
    let sign = if bytes < 0.0 { "-" } else { "" };
    let abs = bytes.abs();

    if abs < 1024.0 {
        format!("{sign}{abs:.0} B")
    } else if abs < 1024.0 * 1024.0 {
        format!("{sign}{:.1} KiB", abs / 1024.0)
    } else {
        format!("{sign}{:.2} MiB", abs / (1024.0 * 1024.0))
    }
}

fn main() {
    let iterations = std::env::var("ALLOC_PROFILE_ITERATIONS")
        .ok()
        .and_then(|value| value.parse().ok())
        .filter(|value| *value > 0)
        .unwrap_or(100);
    let warmup = std::env::var("ALLOC_PROFILE_WARMUP")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(10);
    let print_buckets =
        std::env::var("ALLOC_PROFILE_BUCKETS").is_ok_and(|value| value == "1" || value.eq_ignore_ascii_case("true"));
    let data_dir = get_test_data_dir();
    let ctx = ProfileContext::new();

    println!("=== Query Compiler Allocation Profile ===");
    println!("Iterations: {iterations}");
    println!("Warmup: {warmup}");
    println!("Queries: {}", query_names().join(", "));
    println!("Buckets: {}", if print_buckets { "on" } else { "off" });
    println!();

    for name in query_names() {
        let query = load_query(&data_dir, &name);

        for _ in 0..warmup {
            black_box(ctx.compile_query(&query));
        }

        println!("--- {name} ---");
        print_phase(
            measure_phase(iterations, || measure_parse(&ctx, &query)),
            "parse_json",
            print_buckets,
        );
        print_phase(
            measure_phase(iterations, || measure_into_doc(&ctx, &query)),
            "into_doc",
            print_buckets,
        );
        print_phase(
            measure_phase(iterations, || measure_graph_build(&ctx, &query)),
            "graph_build",
            print_buckets,
        );
        print_phase(
            measure_phase(iterations, || measure_translate(&ctx, &query)),
            "translate_ir",
            print_buckets,
        );
        print_phase(
            measure_phase(iterations, || measure_compile(&ctx, &query)),
            "compile_ir",
            print_buckets,
        );
        print_phase(
            measure_phase(iterations, || measure_full(&ctx, &query)),
            "full_compile",
            print_buckets,
        );
        print_phase(
            measure_phase(iterations, || measure_serialize(&ctx, &query)),
            "serialize_json",
            print_buckets,
        );
        println!();
    }
}
