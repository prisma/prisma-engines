#!/bin/bash

set -e

pipeline=$(printf "
steps:
    - label: \":rust: Cargo test\"
      command: cd .. && docker run -w /build -v $(pwd):/build prismagraphql/rust-build:latest cargo test

    - label: \":rust: Publish Rustdoc\"
      command: cd .. && ./.buildkite/publish_rustdoc.sh
      branches: master
")

echo "$pipeline" | buildkite-agent pipeline upload