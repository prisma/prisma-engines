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

## Local Dev with Language-Tools
When implementing features for `language-tools` in `prisma-engines`, to sync with your local dev environment for the `language-server`, one can do the following:

### On first setup
```
# Install the latest Rust version with `rustup`
# or update the latest Rust version with `rustup`
rustup update
rustup target add wasm32-unknown-unknown
cargo update -p wasm-bindgen
# Check the version defined in `prisma-schema-wasm/cargo.toml` for `wasm-bindgen` and replace `version` below:
cargo install -f wasm-bindgen-cli@version
```

### On Changes

```bash
./prisma-schema-wasm/scripts/update-schema-wasm.sh
```

This script has the following expectations:
- `language-tools` is in the same dir as `prisma-engines`
  - i.e. `dir/{prisma-engines,language-tools}`
- it's run in the `prisma-engines` root folder
