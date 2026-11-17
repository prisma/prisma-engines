# @prisma-kb/prisma-schema-wasm

Wasm build of Prisma's schema parser, validator, formatter, and language-tools
APIs for the Prisma Kingbase distribution. It supports the Kingbase MySQL and
Kingbase Oracle providers implemented in this fork.

This is a community-maintained package, not an official Prisma release. Its
public API follows Prisma's internal schema-Wasm API and can change between
releases.

## Install

```bash
npm install @prisma-kb/prisma-schema-wasm
```

## Verify the module loads

```bash
node -e "const schema = require('@prisma-kb/prisma-schema-wasm'); console.log(typeof schema.format)"
```

The command prints `function` when Node can load the generated Wasm module.

## Source and license

Source code and Kingbase-specific changes are maintained at
<https://github.com/lifeng5210/prisma-engines>. The package is distributed
under the Apache-2.0 license, consistent with its Prisma Engines source base.

## Local build

Use the Rust toolchain pinned in `rust-toolchain.toml`. From the repository root:

```sh
rustup target add wasm32-unknown-unknown
```

Install `wasm-bindgen-cli` at the version of `wasm-bindgen` resolved in
`Cargo.lock`, using `cargo install --locked wasm-bindgen-cli --version <matching-version>`
with `<matching-version>` replaced by that version. The packaging step also
requires `jq`.

Build the Node.js package from the repository root:

```sh
make build-schema-wasm
```

The package is written to `target/prisma-schema-wasm`. `PROFILE` selects the
Cargo build profile (default: `dev`), and `SCHEMA_WASM_VERSION` sets the package
version (default: `0.0.0`).

For language-server development, install this built package into the
language-server project that consumes the schema APIs. The installation path
and dependency name must match that project's package configuration.
