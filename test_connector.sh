#!/bin/bash
# TODO: abort if connector argument is missing
set -e
export CONNECTOR_TO_TEST=$1
export ABSOLUTE_CARGO_TARGET_DIR=`realpath $CARGO_TARGET_DIR`
export IS_DEBUG_BUILD=0
# this must be in sync with the setting in the engineer build CLI
export SQLITE_MAX_VARIABLE_NUMBER=250000

echo "Will run tests against connector $CONNECTOR_TO_TEST"
echo $CONNECTOR_TO_TEST > current_connector

if [ "$IS_DEBUG_BUILD" = "1" ]
then
  cargo build
else
  cargo build --release
fi

cd query-engine/connector-test-kit
sbt -mem 3072 test