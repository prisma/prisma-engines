#!/usr/bin/env bash

set -euo pipefail

if [[ -z "${WASM_BUILD_PROFILE:-}" ]]; then
    WASM_BUILD_PROFILE="release"
fi

if [[ -z "${NPM_PACKAGE_VERSION:-}" ]]; then
    NPM_PACKAGE_VERSION="0.0.0"
fi

if [[ $WASM_BUILD_PROFILE == "dev" ]]; then
    TARGET_DIR="debug"
else
    TARGET_DIR=$WASM_BUILD_PROFILE
fi

printf '%s\n' "entering install.sh"

printf '%s\n' " -> Creating out dir..."
# shellcheck disable=SC2154
mkdir -p "$out"/src

printf '%s\n' " -> Copying package.json and updating version to $NPM_PACKAGE_VERSION"
jq ".version = \"$NPM_PACKAGE_VERSION\"" ./prisma-schema-wasm/package.json > "$out/package.json"

printf '%s\n' " -> Copying README.md"
cp ./prisma-schema-wasm/README.md "$out"/

printf '%s\n' " -> Generating node package"
wasm-bindgen \
  --target bundler \
  --out-dir "$out"/src \
  "target/wasm32-unknown-unknown/$TARGET_DIR/prisma_schema_build.wasm"
