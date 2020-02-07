#!/bin/bash
# TODO: abort if connector argument is missing
set -e
export CONNECTOR_TO_TEST=$1
ABSOLUTE_CARGO_TARGET_DIR=`realpath $CARGO_TARGET_DIR`
export PRISMA_BINARY_PATH=${ABSOLUTE_CARGO_TARGET_DIR}/debug/prisma
export MIGRATION_ENGINE_BINARY_PATH=${ABSOLUTE_CARGO_TARGET_DIR}/debug/migration-engine

echo "Will run tests against connector $CONNECTOR_TO_TEST"
echo $CONNECTOR_TO_TEST > current_connector

cargo build

cd query-engine/connector-test-kit
sbt -mem 3072 test