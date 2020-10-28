#!/bin/bash

set -e

pipeline=$(printf "
steps:
    - label: \":rust: All features\"
      command: ./.buildkite/docker.sh \"cargo test --features=all\"

    - label: \":sqlite: Sqlite minimal\"
      command: ./.buildkite/docker.sh \"cargo test --lib --no-default-features --features=sqlite\"

    - label: \":sqlite: Sqlite full\"
      command: ./.buildkite/docker.sh \"cargo test --lib --no-default-features --features=sqlite,chrono,json,uuid,pooled,serde-support\"

    - label: \":postgres: PostgreSQL minimal\"
      command: ./.buildkite/docker.sh \"cargo test --lib --no-default-features --features=postgresql\"

    - label: \":postgres: PostgreSQL full\"
      command: ./.buildkite/docker.sh \"cargo test --lib --no-default-features --features=postgresql,chrono,json,uuid,pooled,serde-support\"

    - label: \":mysql: MySQL minimal\"
      command: ./.buildkite/docker.sh \"cargo test --lib --no-default-features --features=mysql\"

    - label: \":mysql: MySQL full\"
      command: ./.buildkite/docker.sh \"cargo test --lib --no-default-features --features=mysql,chrono,json,uuid,pooled,serde-support\"

    - label: \":windows: SQL Server minimal\"
      command: ./.buildkite/docker.sh \"cargo test --lib --no-default-features --features=mssql\"

    - label: \":windows: SQL Server full\"
      command: ./.buildkite/docker.sh \"cargo test --lib --no-default-features --features=mssql,chrono,json,uuid,pooled,serde-support\"

    - wait

    - label: \":rust: Publish Rustdoc\"
      command: ./.buildkite/publish_rustdoc.sh
      branches: master
")

echo "$pipeline" | buildkite-agent pipeline upload
