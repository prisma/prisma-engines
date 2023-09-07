#!/usr/bin/env bash

SOURCE=$(dirname ${BASH_SOURCE[0]})
echo -n "source: "
echo $SOURCE
node "${SOURCE}/query-engine/driver-adapters/js/connector-test-kit-executor/dist/index.mjs"
