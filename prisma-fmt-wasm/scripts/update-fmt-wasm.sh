#!/usr/bin/env bash

set -euo pipefail

## Dirs
prisma_fmt_wasm_dir="./prisma-fmt-wasm"
language_tools="../language-tools/"
language_tools_server="$language_tools/packages/language-server/"
language_tools_node="$language_tools_server/node_modules/@prisma/prisma-fmt-wasm/"

[ ! -d "$language_tools" ] && echo "language-tools was not found at the same level as engines" && exit 1

[ ! -d "$prisma_fmt_wasm_dir" ] && echo "prisma_fmt_wasm was not found in the current directory" && exit 1

## Script
printf '%s\n' "Starting build :: prisma-fmt-wasm"
cargo build --release --target=wasm32-unknown-unknown --manifest-path=$prisma_fmt_wasm_dir/Cargo.toml

printf '%s\n' "Marking executables :: $prisma_fmt_wasm_dir/scripts/*"
chmod +x $prisma_fmt_wasm_dir/scripts/*

printf '%s\n' "Generating node module"
out=$prisma_fmt_wasm_dir/nodejs $prisma_fmt_wasm_dir/scripts/install.sh

printf '%s\n' "Removing pre-existing wasm in language-tools"
rm -rf $language_tools_node/src/*

printf '%s\n' "Moving generated prisma-fmt-wasm :: engines -> language-tools"
cp $prisma_fmt_wasm_dir/nodejs/src/prisma_fmt_build{_bg.wasm,_bg.wasm.d.ts,.d.ts,.js} $language_tools_node/src
