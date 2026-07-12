#!/usr/bin/env bash

set -euo pipefail

## Dirs
prisma_schema_wasm_dir="./prisma-schema-wasm"
language_tools="../language-tools/"
language_tools_server="$language_tools/packages/language-server/"
language_tools_node="$language_tools_server/node_modules/@prisma/prisma-schema-wasm/"

[ ! -d "$language_tools" ] && echo "language-tools was not found at the same level as engines" && exit 1

[ ! -d "$prisma_schema_wasm_dir" ] && echo "prisma_schema_wasm was not found in the current directory" && exit 1

## Script
printf '%s\n' "Starting build :: prisma-schema-wasm"
cargo build --features wasm-logger --release --target=wasm32-unknown-unknown --manifest-path=$prisma_schema_wasm_dir/Cargo.toml

printf '%s\n' "Generating node module"
out=$prisma_schema_wasm_dir/nodejs $prisma_schema_wasm_dir/scripts/install.sh

printf '%s\n' "Removing pre-existing wasm in language-tools"
rm -rf $language_tools_node/src/*

printf '%s\n' "Moving generated prisma-schema-wasm :: engines -> language-tools"
cp $prisma_schema_wasm_dir/nodejs/src/prisma_schema_build{_bg.wasm,_bg.wasm.d.ts,.d.ts,.js} $language_tools_node/src
