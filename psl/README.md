# Prisma Schema Language implementation

This directory contains the crates responsible for parsing, validating and
formatting Prisma schemas. The entrypoint is [psl](./psl), it is the public
API for other parts of prisma-engines.

## Organization

The crate dependency graph is and should remain simple:

[diagnostics](./diagnostics) →
[schema-ast](./schema-ast) →
[parser-database](./parser-database) →
[psl-core](./psl-core) →
[psl](./psl)

The built-in connectors live inside
[psl-core/src/builtin_connectors](./psl-core/src/builtin_connectors), rather
than in a separate connector crate. Connector features are forwarded by the public `psl` crate.

Query Compiler's [InternalDataModel](../query-compiler/query-structure/src/internal_data_model.rs)
wraps a validated PSL schema. There is no separate `psl/connectors/dml` crate.
