#!/bin/bash

SCRIPT_DIR="$( cd "$(dirname "$0")" ; pwd -P )"

# Change directory to the specified location
cd $SCRIPT_DIR/query-engine/driver-adapters/js/

# Install dependencies and build using pnpm
pnpm i && pnpm build

# Build the node api
cd $SCRIPT_DIR
cargo build -p query-engine-node-api

# Run cargo test for query-engine-tests
export NODE_TEST_EXECUTOR=$SCRIPT_DIR/start.sh
export RUST_BACKTRACE=1
cargo test -p query-engine-tests "$@"
