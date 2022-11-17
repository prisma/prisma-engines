#!/usr/bin/env bash

hyperfine --warmup 1 --max-runs 3  -p 'cargo clean -p query-engine-tests' 'cargo build --tests' --export-markdown=compile-bench.md
