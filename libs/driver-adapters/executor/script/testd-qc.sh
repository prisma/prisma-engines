#!/usr/bin/env bash

DEBUG_FLAGS=""

if [ -n "$QC_RUNNER_INSPECT_BRK" ]; then
  DEBUG_FLAGS="--inspect-brk --experimental-worker-inspection"
elif [ -n "$QC_RUNNER_INSPECT" ]; then
  DEBUG_FLAGS="--inspect --experimental-worker-inspection"
fi

# shellcheck disable=SC2086
node $DEBUG_FLAGS "$(dirname "${BASH_SOURCE[0]}")/../dist/qc-test-runner.js"
