#!/bin/bash
set -eux;

# full command
command="docker run \
-e SQLITE_MAX_VARIABLE_NUMBER=250000 \
-e SQLITE_MAX_EXPR_DEPTH=10000 \
-e LIBZ_SYS_STATIC=1 \
-w /root/build \
-v \"$(pwd)\":/root/build \
-v \"$HOME\"/.cargo/bin:/root/cargo/bin \
-v \"$HOME\"/.cargo/registry/index:/root/cargo/registry/index \
-v \"$HOME\"/.cargo/registry/cache:/root/cargo/registry/cache \
-v \"$HOME\"/.cargo/git/db:/root/cargo/git/db \
$IMAGE \
bash -c \
    \" \
    cargo clean \
    && cargo build --release -p query-engine          --manifest-path query-engine/query-engine/Cargo.toml          $TARGET_STRING $FEATURES_STRING \
    && cargo build --release -p query-engine-node-api --manifest-path query-engine/query-engine-node-api/Cargo.toml $TARGET_STRING $FEATURES_STRING \
    && cargo build --release -p schema-engine-cli     --manifest-path schema-engine/cli/Cargo.toml                  $TARGET_STRING $FEATURES_STRING \
    && cargo build --release -p prisma-fmt            --manifest-path prisma-fmt/Cargo.toml                         $TARGET_STRING $FEATURES_STRING \
    \" \
"
# remove query-engine-node-api for "static" targets
if [[ "$TARGET_NAME" == *-static-* ]]; then
    substring_to_replace="&& cargo build --release -p query-engine-node-api --manifest-path query-engine/query-engine-node-api/Cargo.toml $TARGET_STRING $FEATURES_STRING"
    command="${command/$substring_to_replace/}"
fi

echo "$command"