#!/bin/bash
# TODO: abort if connector argument is missing
set -e
export CONNECTOR_TO_TEST=$1
ABSOLUTE_CARGO_TARGET_DIR=`realpath $CARGO_TARGET_DIR`
export PRISMA_BINARY_PATH=${ABSOLUTE_CARGO_TARGET_DIR}/release/prisma
export MIGRATION_ENGINE_BINARY_PATH=${ABSOLUTE_CARGO_TARGET_DIR}/release/migration-engine

echo "Will run tests against connector $CONNECTOR_TO_TEST"
echo $CONNECTOR_TO_TEST > current_connector

cargo build --release

cd query-engine/connector-test-kit
sbt -mem 3072 test