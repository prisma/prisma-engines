#!/usr/bin/env bash

SOURCE=$(dirname ${BASH_SOURCE[0]})
node "${SOURCE}/query-engine/js-connectors/js/connector-test-kit-executor/dist/index.mjs"
