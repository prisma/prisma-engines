# @prisma/schema-engine-wasm

[![Publish pipeline](https://github.com/prisma/prisma-engines/actions/workflows/publish-prisma-schema-engine-wasm.yml/badge.svg)](https://github.com/prisma/prisma-engines/actions/workflows/publish-prisma-schema-engine-wasm.yml)
[![npm package](https://img.shields.io/npm/v/@prisma/schema-engine-wasm/latest)](https://www.npmjs.com/package/@prisma/schema-engine-wasm)
[![install size](https://packagephobia.com/badge?p=@prisma/schema-engine-wasm)](https://packagephobia.com/result?p=@prisma/schema-engine-wasm)

This project exposes WebAssembly bindings for the Prisma Schema Engine, which is used by Prisma to perform database migrations and introspections.

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

From the root of the repository:

- `SCHEMA_WASM_VERSION=$OUT_NPM_VERSION make build-se-wasm`

By default, the build profile is `dev` in local builds, and `release` in CI builds.
You can override this by setting the `WASM_BUILD_PROFILE` environment variable.
E.g., `WASM_BUILD_PROFILE="profiling"` is useful for interoperating with the `twiggy` size profiler.

See [`./build.sh`](./build.sh) for more details.

## Example

Using Node.js 20.9.0:

```bash
❯ node -e "const { version } = await import('@prisma/schema-engine-wasm'); console.log(version())" 
be55204b41e1041f71e247fe0af67399bd34653d
```
