#!/usr/bin/env bash
export DATABASE_URL="postgresql://postgres:postgres@localhost:5432/bench"
cat $(dirname "${BASH_SOURCE[0]}")/../bench/schema.prisma | node --experimental-wasm-modules $(dirname "${BASH_SOURCE[0]}")/../dist/bench.mjs