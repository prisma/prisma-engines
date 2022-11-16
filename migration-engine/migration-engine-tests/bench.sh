#!/usr/bin/env bash
hyperfine -w 2 -p 'cargo clean -p migration-engine-tests' 'cargo build --tests' --export-markdown=compile-bench.md
