# Query Compiler Benchmarking and Profiling Guide

This document describes how to run benchmarks and profile the query compiler to identify performance bottlenecks.

## Quick Start

```bash
# Run all query-compiler benchmarks
cargo bench -p query-compiler

# Or use the Makefile target
make bench-qc

# Run benchmarks matching a pattern
cargo bench -p query-compiler -- "create"
cargo bench -p query-compiler -- "query-m2o"
```

## How Benchmarks Work

The compilation benchmarks automatically discover all `.json` query files in `tests/data/` and create a benchmark for each one. This means:

- **No code changes needed** when adding new test queries
- Benchmarks stay in sync with unit tests
- Each benchmark measures full end-to-end compilation (JSON → Expression)

The benchmarks use the same schema (`tests/data/schema.prisma`) as the unit tests.

## Running Benchmarks

### Basic Usage

```bash
# Run all benchmarks
cargo bench -p query-compiler

# Run benchmarks matching a pattern
cargo bench -p query-compiler -- "compile/create"
cargo bench -p query-compiler -- "compile/query-m2o"

# Save baseline for comparison
cargo bench -p query-compiler -- --save-baseline main

# Compare against baseline
cargo bench -p query-compiler -- --baseline main
```

### Makefile Targets

```bash
make bench-qc                    # Run query compiler benchmarks
make bench-qc-graph              # Run query graph benchmarks
make bench-schema                # Run schema building benchmarks
make bench-baseline NAME=main    # Save baseline
make bench-compare NAME=main     # Compare against baseline
make profile-qc                  # Run profiling example
```

### Benchmark Options

```bash
# Run with more iterations for accuracy
cargo bench -p query-compiler -- --sample-size 200

# Run for a specific duration
cargo bench -p query-compiler -- --measurement-time 30

# Skip warmup (useful for debugging)
cargo bench -p query-compiler -- --warm-up-time 0

# List all available benchmarks
cargo bench -p query-compiler -- --list
```

## Adding New Benchmarks

Simply add a new `.json` query file to `tests/data/`. The benchmark will be automatically discovered on the next run.

For example, adding `tests/data/my-new-query.json` will create a benchmark named `compile/my-new-query`.

## Profiling

### Using the Profiling Example

The `profile_query` example is designed for use with profilers:

```bash
# Run with default settings (10,000 iterations)
cargo run -p query-compiler --example profile_query --profile profiling

# Customize via environment variables
PROFILE_ITERATIONS=50000 PROFILE_QUERY=nested cargo run -p query-compiler --example profile_query --profile profiling
```

Environment variables:
- `PROFILE_ITERATIONS`: Number of iterations (default: 10000)
- `PROFILE_QUERY`: Which query to profile (`simple`, `nested`, `mutation`, or `all`)
- `PROFILE_WARMUP`: Number of warmup iterations (default: 100)

### Using samply (Recommended, Cross-platform)

```bash
cargo install samply

# Profile the example
samply record cargo run -p query-compiler --example profile_query --profiling

# Opens Firefox Profiler with results
```

### Using Instruments.app (macOS)

```bash
cargo build -p query-compiler --example profile_query --profile profiling

xcrun xctrace record --template 'Time Profiler' --launch -- \
    ./target/profiling/examples/profile_query
```

### Using perf (Linux)

```bash
cargo build -p query-compiler --example profile_query --profile profiling

perf record -g ./target/profiling/examples/profile_query
perf report
```

### Using flamegraph

```bash
# macOS
brew install flamegraph

# Linux
cargo install flamegraph

# Generate flamegraph
cargo flamegraph -p query-compiler --example profile_query
```

## Interpreting Results

### Criterion Output

```
compile/query-m2o       time:   [143.02 µs 156.65 µs 173.14 µs]
                        change: [-2.12% -0.57% +1.01%] (p = 0.42 > 0.05)
                        No change in performance detected.
```

- **time**: [lower bound, estimate, upper bound] at 95% confidence
- **change**: Comparison to previous run or baseline
- **p-value**: Statistical significance (< 0.05 means significant change)

### Performance Regression Detection

- Changes > 5% warrant investigation
- Changes > 10% are likely regressions
- Always verify with multiple runs

## Other Benchmark Suites

- `query-compiler/core-tests/benches/query_graph_bench.rs` - Query graph building
- `query-compiler/schema/benches/schema_builder_bench.rs` - Schema building

## Troubleshooting

### Benchmarks are too slow

```bash
# Use fewer samples for quick runs
cargo bench -p query-compiler -- --sample-size 10

# Filter to specific benchmarks
cargo bench -p query-compiler -- "compile/query-m2o"
```

### High variance in results

- Close other applications
- Use `--warm-up-time 5` for longer warmup
- Use `--sample-size 500` for more samples

### Missing debug symbols in profiles

Use the `profiling` profile defined in the workspace `Cargo.toml`:

```bash
cargo build -p query-compiler --profile profiling
```
