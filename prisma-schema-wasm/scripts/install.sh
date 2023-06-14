#!/usr/bin/env bash

set -euo pipefail

printf '%s\n' "entering install.sh"

printf '%s\n' " -> Creating out dir..."
# shellcheck disable=SC2154
mkdir -p "$out"/src

printf '%s\n' " -> Copying package.json"
cp ./prisma-schema-wasm/package.json "$out"/

printf '%s\n' " -> Copying README.md"
cp ./prisma-schema-wasm/README.md "$out"/

printf '%s\n' " -> Generating node package"
wasm-bindgen \
  --target nodejs \
  --out-dir "$out"/src \
  target/wasm32-unknown-unknown/release/prisma_schema_build.wasm
