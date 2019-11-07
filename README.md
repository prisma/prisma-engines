# Prisma Engines

The core stack for [Photon](https://github.com/prisma/photonjs/) and
[Lift](https://github.com/prisma/lift/).

### Code architecture

Photon uses query-engine and its
[main](https://github.com/prisma/prisma-engine/blob/master/query-engine/prisma/src/main.rs)
is a good place to start digging into the code.

The request basically flows from
[server.rs](https://github.com/prisma/prisma-engine/blob/master/query-engine/prisma/src/server.rs)
to [graphql
handler](https://github.com/prisma/prisma-engine/blob/master/query-engine/prisma/src/request_handlers/graphql/handler.rs)
and from there to [core
executor](https://github.com/prisma/prisma-engine/blob/master/query-engine/core/src/executor/interpreting_executor.rs)
down to the
[connectors](https://github.com/prisma/prisma-engine/tree/master/query-engine/connectors/sql-query-connector/src).

The SQL generation and SQL database abstractions are handled by the [quaint
crate](https://github.com/prisma/quaint).

Lift connects to the migration-engine and the starting point for requests is the
[rpc
api](https://github.com/prisma/prisma-engine/blob/master/migration-engine/core/src/api/rpc.rs).

### Coding Guidelines

* Prevent compiler warnings
* Use Rust formatting (`cargo fmt`)

### Testing

* To compile all modules use the provided `build.sh` script
* To test all modules use the provided `test.sh` script
