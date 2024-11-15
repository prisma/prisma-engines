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
By default, the build profile is `dev` in local builds, and `release` in CI builds.
You can override this by setting the `WASM_BUILD_PROFILE` environment variable.
E.g., `WASM_BUILD_PROFILE="profiling"` is useful for interoperating with the `twiggy` size profiler.

See [`./build.sh`](./build.sh) for more details.

## How to Publish

From the current folder:

- `wasm-pack publish --access public`

## How to Test

To try importing the , you can run:

- `nvm use`
- `node --experimental-wasm-modules example/example.js`

## How to analyse the size of the Wasm binary

- Build the Wasm binary with `WASM_BUILD_PROFILE="profiling" ./build.sh "0.0.1"`
- Run `twiggy top -n 20 ./pkg/query_engine_bg.wasm`
- Take a look at this [Notion document](https://www.notion.so/prismaio/Edge-Functions-how-to-use-twiggy-and-other-size-tracking-tools-c1cb481cbd0c4a0488f6876674988382) for more details, and for instructions on how to refine `twiggy`'s output via [`./wasm-inspect.sh`](./wasm-inspect.sh).

## How to analyse the size impact of Rust crates on the Wasm binary

Please refer to the `pnpm crates` command in the [`./analyse`](./analyse/README.md) README.
