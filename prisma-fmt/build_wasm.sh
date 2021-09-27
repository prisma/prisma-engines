#!/usr/bin/env bash

set -euo pipefail

export builddir=`mktemp -d`

echo "Building the .wasm artifact..."
cargo build --lib --target=wasm32-unknown-unknown --target-dir=$builddir --release

echo "Installing wasm-bindgen-cli..."
cargo install wasm-bindgen-cli --version=0.2.78

echo "Building npm package"
wasm-bindgen --target=nodejs $builddir/wasm32-unknown-unknown/release/prisma_fmt.wasm --out-dir=pkg/src/

echo "ok"
