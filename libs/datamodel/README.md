# Prisma Schema Language implementation

This directory contains the crates responsible for parsing, validating and
formatting Prisma schemas. The entrypoint and what should be considered public
API is [core](./core).

## Organization

The crate graph is moving towards a completely linear dependency graph:

[schema-ast](./schema-ast) →
[parser-database](./parser-database) →
[datamodel-connector](./connectors/datamodel-connector) →
[core](./core)

[dml](./connectors/dml) is a separate data structure that can optionally be
produced ("lifted") by [core](./core).

We are getting close, but this is currently aspirational, and still in flux:
- [datamodel-connector](./connectors/datamodel-connector) still depends on
[dml](./connectors/dml) 
- AST reformatting still depends on dml
- The reexports from other crates should be more principled.

