# PSL stands for Prisma Schema Language

## Overview

This crate is the public API for working with Prisma schemas: parsing and
validating their string representation, analyzing them, providing convenient
APIs, rendering, reformatting, etc. It is the main implementation of the Prisma
Schema Language, and it is used by all Prisma engines in this repository.

The query compiler further processes Prisma schemas to produce the client API,
and the DMMF JSON format.

## Usage

Please see [`lib.rs`](src/lib.rs) and the [rustdoc documentation](https://prisma.github.io/prisma-engines/doc/psl/) for the public API.

For a schema without extension types, parse and validate it with:

```rust,no_run
let schema = "model User {\n  id Int @id\n}";
let validated_schema = psl::parse_schema_without_extensions(schema).unwrap();
```

Use `psl::parse_schema(schema, extension_types)` when the caller supplies
extension types. Enable the connector features needed by datasource providers;
see this crate's `Cargo.toml` for individual features and the `all` feature.
