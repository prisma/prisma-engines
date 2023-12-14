#!/usr/bin/env bash
export TEST_DATABASE_URL="postgresql://postgres:prisma@localhost:5435"

cat "$(dirname "${BASH_SOURCE[0]}")/../src/bench/schema.prisma | node "$(dirname "${BASH_SOURCE[0]}")/../dist/bench.mjs"