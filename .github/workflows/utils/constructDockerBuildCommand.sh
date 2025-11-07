#!/bin/bash
set -eux;

DOCKER_WORKSPACE="/root/build"

# Full command, Docker + Bash.
# In Bash, we use `git config` to avoid "fatal: detected dubious ownership in repository at /root/build" panic messages
# that can occur when Prisma Engines' `build.rs` scripts run `git rev-parse HEAD` to extract the current commit hash.
# See: https://www.kenmuse.com/blog/avoiding-dubious-ownership-in-dev-containers/.
command="docker run \
-e SQLITE_MAX_VARIABLE_NUMBER=250000 \
-e SQLITE_MAX_EXPR_DEPTH=10000 \
-e LIBZ_SYS_STATIC=1 \
-w ${DOCKER_WORKSPACE} \
-v \"$(pwd)\":${DOCKER_WORKSPACE} \
-v \"$HOME\"/.cargo/bin:/root/cargo/bin \
-v \"$HOME\"/.cargo/registry/index:/root/cargo/registry/index \
-v \"$HOME\"/.cargo/registry/cache:/root/cargo/registry/cache \
-v \"$HOME\"/.cargo/git/db:/root/cargo/git/db \
$IMAGE \
bash -c \
    \" \
    git config --global --add safe.directory ${DOCKER_WORKSPACE} \
    && cargo clean \
    && cargo build --release -p schema-engine-cli --manifest-path schema-engine/cli/Cargo.toml $TARGET_STRING $FEATURES_STRING \
    && cargo build --release -p prisma-fmt        --manifest-path prisma-fmt/Cargo.toml        $TARGET_STRING $FEATURES_STRING \
    \" \
"
echo "$command"
