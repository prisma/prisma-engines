# @prisma/query-engine-wasm

**INTERNAL PACKAGE, DO NOT USE**

This is a Wasm-compatible version of the Query Engine library (libquery).
Currently, it just contains a skeleton of the public API, as some internal crates are still not Wasm-compatible.

The published npm package is internal to Prisma. Its API will break without prior warning.

## Setup

```
# Install the latest Rust version with `rustup`
# or update the latest Rust version with `rustup`
rustup update
rustup target add wasm32-unknown-unknown
cargo install wasm-bindgen
cargo install wasm-pack
```

## How to Build

From the current folder:

- `./build.sh $OUT_NPM_VERSION`

where e.g. `OUT_NPM_VERSION="0.0.1"` is the version you want to publish this package on npm with.

## How to Publish

From the current folder:

- `wasm-pack publish --access public`

## How to Test

To try importing the , you can run:

- `nvm use`
- `node --experimental-wasm-modules example/example.js`
