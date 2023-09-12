#!/bin/bash

SCRIPT_DIR="$(dirname "${BASH_SOURCE[0]}")"

# Change directory to the specified location
cd $SCRIPT_DIR/query-engine/driver-adapters/js/connector-test-kit-executor/

# Install dependencies and build using pnpm
pnpm i && pnpm build

# Export NODE_TEST_EXECUTOR environment variable
cd $SCRIPT_DIR

# Run cargo test for query-engine-tests
export NODE_TEST_EXECUTOR=$SCRIPT_DIR/start.sh
cargo test -p query-engine-tests query-engine-tests
