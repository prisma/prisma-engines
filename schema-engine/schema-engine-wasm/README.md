# @prisma/schema-engine-wasm

[![Publish pipeline](https://github.com/prisma/prisma-engines/actions/workflows/publish-schema-engine-wasm.yml/badge.svg)](https://github.com/prisma/prisma-engines/actions/workflows/publish-schema-engine-wasm.yml)
[![npm package](https://img.shields.io/npm/v/@prisma/schema-engine-wasm/latest)](https://www.npmjs.com/package/@prisma/schema-engine-wasm)
[![install size](https://packagephobia.com/badge?p=@prisma/schema-engine-wasm)](https://packagephobia.com/result?p=@prisma/schema-engine-wasm)

This project exposes WebAssembly bindings for the Prisma Schema Engine, which is used by Prisma to perform database migrations and introspections.

The published npm package is internal to Prisma. Its API will break without prior warning.

## Setup

Use the Rust toolchain pinned in the repository's `rust-toolchain.toml`.
Run setup commands from the repository root:

```sh
rustup target add wasm32-unknown-unknown
```

Install `wasm-bindgen-cli` at the version matching the resolved `wasm-bindgen`
dependency in `Cargo.lock`, for example
`cargo install --locked wasm-bindgen-cli --version <matching-version>`.
The `wasm-bindgen` crate itself is a library, not the CLI package.
Release/profiling optimization also requires the Binaryen `wasm-opt` used by
`build.sh`; check that script before selecting the build profile.

## How to Build

From the current folder:

- `./build.sh $OUT_NPM_VERSION`

where e.g. `OUT_NPM_VERSION="0.0.1"` is the version you want to publish this package on npm with.

From the root of the repository:

- `SCHEMA_ENGINE_WASM_VERSION="$OUT_NPM_VERSION" make build-se-wasm`

`SCHEMA_WASM_VERSION` instead configures `make build-schema-wasm`, the separate
PSL/schema-tools package. It does not set the version for this Schema Engine package.

By default, the build profile is `dev` in local builds, and `release` in CI builds.
You can override this by setting the `WASM_BUILD_PROFILE` environment variable.
E.g., `WASM_BUILD_PROFILE="profiling"` is useful for interoperating with the `twiggy` size profiler.

See [`./build.sh`](./build.sh) for more details.

## Example

Using Node.js 20.9.0:

```bash
❯ node --experimental-wasm-modules -e "const { version } = await import('@prisma/schema-engine-wasm'); console.log(version())" 
(node:4977) ExperimentalWarning: Importing WebAssembly modules is an experimental feature and might change at any time
(Use `node --trace-warnings ...` to show where the warning was created)
4411ab2a4b97a8109bf19c2678742abde9ed60a4
```
