#!/bin/sh

echo "Building WASM from source..."
cargo build --lib --target=wasm32-unknown-unknown --target-dir=wasm-target
echo "Building JS module from WASM..."
wasm-bindgen --target=nodejs ./wasm-target/wasm32-unknown-unknown/debug/prisma_fmt.wasm --out-dir=js-target
echo "ok"
