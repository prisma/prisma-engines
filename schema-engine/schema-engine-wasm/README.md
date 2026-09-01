# @vertexa/prisma-schema-engine-wasm

Fork of [`@prisma/schema-engine-wasm`](https://github.com/prisma/prisma-engines) — WebAssembly bindings for the Prisma Schema Engine, used for database migrations and introspections.

The fork adds PostGIS `Geometry` / `Geography` type support to schema migrations and introspection.

## Internal package

Consumed by `@vertexa/prisma` CLI for `prisma migrate` and `prisma db push` commands.
Do not depend on it directly — install `@vertexa/prisma` instead.

## Build

This package is built from `prisma-engines/schema-engine/schema-engine-wasm/` using `wasm-pack`.
See `docs/fork-publishing.md` for the full build pipeline.

## License

Apache-2.0
