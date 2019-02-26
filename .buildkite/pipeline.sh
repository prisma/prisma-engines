#!/bin/bash

set -e

echo $(pwd)

pipeline=$(printf "
steps:
    - label: \":rust: Cargo test\"
      command: docker run -w /build -v $(pwd):/build prismagraphql/rust-build:latest cargo test

    - wait

    - label: \":rust: Publish Rustdoc\"
      command: ./.buildkite/publish_rustdoc.sh
      branches: master
")

echo "$pipeline" | buildkite-agent pipeline upload