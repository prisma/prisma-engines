//! Example for profiling query compilation.
//!
//! This example can be run with various profilers to identify bottlenecks:
//!
//! ## Using flamegraph
//!
//! ```bash
//! cargo flamegraph -p query-compiler --example profile_query
//! ```
//!
//! ## Using samply (cross-platform, opens Firefox Profiler)
//!
//! ```bash
//! samply record cargo run -p query-compiler --example profile_query --release
//! ```
//!
//! ## Using Instruments.app (macOS)
//!
//! ```bash
//! cargo build -p query-compiler --example profile_query --profile profiling
//! xcrun xctrace record --template 'Time Profiler' --launch -- \
//!     ./target/profiling/examples/profile_query
//! ```
//!
//! ## Using perf (Linux)
//!
//! ```bash
//! cargo build -p query-compiler --example profile_query --profile profiling
//! perf record -g ./target/profiling/examples/profile_query
//! perf report
//! ```
//!
//! ## Environment Variables
//!
//! - `PROFILE_ITERATIONS`: Number of iterations (default: 10000)
//! - `PROFILE_QUERY`: Which query to profile (simple, nested, mutation, all)
//! - `PROFILE_WARMUP`: Number of warmup iterations (default: 100)

use quaint::prelude::{ConnectionInfo, ExternalConnectionInfo, SqlFamily};
use query_compiler::compile;
use request_handlers::{JsonProtocolAdapter, JsonSingleQuery};
use schema::QuerySchema;
use std::sync::Arc;
use std::time::Instant;

const SCHEMA: &str = r#"
datasource db {
    provider = "postgresql"
}

model User {
    id        Int       @id @default(autoincrement())
    email     String    @unique
    name      String?
    posts     Post[]
    profile   Profile?
    createdAt DateTime  @default(now())
    updatedAt DateTime  @updatedAt
}

model Post {
    id        Int       @id @default(autoincrement())
    title     String
    content   String?
    published Boolean   @default(false)
    author    User      @relation(fields: [authorId], references: [id])
    authorId  Int
    comments  Comment[]
    tags      Tag[]
    createdAt DateTime  @default(now())
}

model Comment {
    id        Int      @id @default(autoincrement())
    text      String
    post      Post     @relation(fields: [postId], references: [id])
    postId    Int
    createdAt DateTime @default(now())
}

model Profile {
    id     Int     @id @default(autoincrement())
    bio    String?
    user   User    @relation(fields: [userId], references: [id])
    userId Int     @unique
}

model Tag {
    id    Int    @id @default(autoincrement())
    name  String @unique
    posts Post[]
}
"#;

const QUERY_SIMPLE: &str = r#"
{
    "action": "findMany",
    "modelName": "User",
    "query": {
        "selection": {
            "id": true,
            "email": true,
            "name": true
        }
    }
}
"#;

const QUERY_NESTED: &str = r#"
{
    "action": "findMany",
    "modelName": "User",
    "query": {
        "selection": {
            "id": true,
            "email": true,
            "name": true,
            "posts": {
                "selection": {
                    "id": true,
                    "title": true,
                    "published": true,
                    "comments": {
                        "selection": {
                            "id": true,
                            "text": true
                        }
                    },
                    "tags": {
                        "selection": {
                            "id": true,
                            "name": true
                        }
                    }
                }
            },
            "profile": {
                "selection": {
                    "id": true,
                    "bio": true
                }
            }
        }
    }
}
"#;

const QUERY_MUTATION: &str = r#"
{
    "action": "createOne",
    "modelName": "User",
    "query": {
        "arguments": {
            "data": {
                "email": "test@example.com",
                "name": "Test User",
                "posts": {
                    "create": [
                        {
                            "title": "First Post",
                            "content": "Hello World",
                            "tags": {
                                "connectOrCreate": [
                                    {
                                        "where": { "name": "rust" },
                                        "create": { "name": "rust" }
                                    }
                                ]
                            }
                        }
                    ]
                },
                "profile": {
                    "create": {
                        "bio": "A new user"
                    }
                }
            }
        },
        "selection": {
            "id": true,
            "email": true,
            "posts": {
                "selection": {
                    "id": true,
                    "title": true
                }
            }
        }
    }
}
"#;

struct ProfileContext {
    query_schema: QuerySchema,
    connection_info: ConnectionInfo,
}

impl ProfileContext {
    fn new() -> Self {
        let validated_schema = psl::parse_schema_without_extensions(SCHEMA).unwrap();
        let query_schema = schema::build(Arc::new(validated_schema), true);
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

    #[must_use]
    #[inline(never)]
    fn compile_query(&self, query_json: &str) -> String {
        let json_request: JsonSingleQuery = serde_json::from_str(query_json).unwrap();
        let mut adapter = JsonProtocolAdapter::new(&self.query_schema);
        let operation = adapter.convert_single(json_request).unwrap();

        let mut expression = compile(&self.query_schema, operation, &self.connection_info).unwrap();
        expression.simplify();

        serde_json::to_string(&expression).unwrap()
    }
}

fn main() {
    let iterations: usize = std::env::var("PROFILE_ITERATIONS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(10_000);

    let warmup: usize = std::env::var("PROFILE_WARMUP")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(100);

    let query_type = std::env::var("PROFILE_QUERY").unwrap_or_else(|_| "all".to_string());

    println!("=== Query Compiler Profiling ===");
    println!("Iterations: {}", iterations);
    println!("Warmup: {}", warmup);
    println!("Query type: {}", query_type);
    println!();

    let ctx = ProfileContext::new();

    let queries: Vec<(&str, &str)> = match query_type.as_str() {
        "simple" => vec![("simple", QUERY_SIMPLE)],
        "nested" => vec![("nested", QUERY_NESTED)],
        "mutation" => vec![("mutation", QUERY_MUTATION)],
        _ => vec![
            ("simple", QUERY_SIMPLE),
            ("nested", QUERY_NESTED),
            ("mutation", QUERY_MUTATION),
        ],
    };

    let mut counter = 0usize;

    for (name, query) in &queries {
        println!("--- Profiling: {} ---", name);

        print!("Warming up... ");
        for _ in 0..warmup {
            counter += ctx.compile_query(query).len();
        }
        println!("done");

        print!("Running {} iterations... ", iterations);
        let start = Instant::now();
        for _ in 0..iterations {
            counter += ctx.compile_query(query).len();
        }
        let elapsed = start.elapsed();
        println!("done: {counter}");

        let per_iter = elapsed / iterations as u32;
        println!(
            "Total: {:?}, Per iteration: {:?} ({:.2} ops/sec)",
            elapsed,
            per_iter,
            1_000_000_000.0 / per_iter.as_nanos() as f64
        );
        println!();
    }

    println!("=== Profiling Complete ===");
    println!("Attach a profiler to this process for detailed analysis.");
}
