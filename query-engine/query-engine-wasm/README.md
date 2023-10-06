# @prisma/prisma-schema-wasm

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

- `./build.sh`

## How to Publish

From the current folder:

- `wasm-pack publish --tag alpha --access public`

## How to Test

To try importing the , you can run:

- `nvm use`
- `node --experimental-wasm-modules ./example.js`
