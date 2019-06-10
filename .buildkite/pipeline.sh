#!/bin/bash

set -e

pipeline=$(printf "
steps:
    - label: \":rust: Cargo test\"
      command: ./.buildkite/docker.sh

    - wait

    - label: \":rust: Publish Rustdoc\"
      command: ./.buildkite/publish_rustdoc.sh
      branches: master
")

echo "$pipeline" | buildkite-agent pipeline upload
