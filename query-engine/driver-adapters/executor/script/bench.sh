#!/usr/bin/env bash
export DATABASE_URL="postgresql://postgres:postgres@localhost:5432/bench?schema=imdb_bench&sslmode=disable"
node --experimental-wasm-modules "$(dirname "${BASH_SOURCE[0]}")"/../dist/bench.mjs < "$(dirname "${BASH_SOURCE[0]}")"/../bench/schema.prisma