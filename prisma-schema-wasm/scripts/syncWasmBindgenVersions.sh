#!/usr/bin/env bash

echo 'Syncing wasm-bindgen version in crate with that of the installed CLI...'
sed -i "s/^wasm-bindgen\ =.*$/wasm-bindgen = \"=$WASM_BINDGEN_VERSION\"/" ./prisma-schema-wasm/Cargo.toml
cargo update --package wasm-bindgen
