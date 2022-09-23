# Prisma Schema Language implementation

This directory contains the crates responsible for parsing, validating and
formatting Prisma schemas. The entrypoint is [psl](./psl), it is the public
API for other parts of prisma-engines.

## Organization

The crate dependency graph is and should remain simple:

[diagnostics](./diagnostics) →
[schema-ast](./schema-ast) →
[parser-database](./parser-database) →
[psl-core](./psl-core)
[builtin-connectors](./connectors/builtin-connectors) →
[psl](./connectors/psl)

[dml](./connectors/dml) is a separate data structure that can optionally be
produced ("lifted") from a validated schema.
