#!/bin/bash

set -e

pipeline=$(printf "
steps:
    - label: \":rust: Cargo test\"
      command: cd .. && docker run -w /build -v $(pwd):/build prismagraphql/rust-build:latest cargo test
")

echo "$pipeline" | buildkite-agent pipeline upload