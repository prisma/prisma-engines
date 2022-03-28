# Prisma Schema Language implementation

This directory contains the crates responsible for parsing, validating and
formatting Prisma schemas. The entrypoint and what should be considered public
API is [core](./core).

## Organization

The crate dependency graph is and should remain completely linear:

[schema-ast](./schema-ast) →
[parser-database](./parser-database) →
[datamodel-connector](./connectors/datamodel-connector) →
[core](./core)

[dml](./connectors/dml) is a separate data structure that can optionally be
produced ("lifted") by [core](./core).
