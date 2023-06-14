# @prisma/nodejs-playground

This is a playground for testing the `libquery` client with the experimental Node.js drivers.
It contains a subset of `@prisma/client`, plus a handy [`index.ts`](./src/index.ts) file with a `main` function.

## How to use

In the root directory:
  - Run `cargo build -p query-engine-node-api` to compile the `libquery` Query Engine
  - Spawn a MySQL8 database via `make dev-mysql8`
  - Store the `export TEST_DATABASE_URL="mysql://root:prisma@localhost:3307/test"` env var in `.envrc.local` and expose it via `direnv allow .`

In the current directory
  - Copy the content of [`./src/index.sql`](./src/index.sql) into the MySQL8 database available at `mysql://root:prisma@localhost:3307/test`
  - Run `pnpm i` to install dependencies
  - Run `pnpm dev` to run the playground

## How to test

There is no automatic test. However, [./src/index.ts](./src/index.ts) includes a pipeline you can use to interactively experiment with the new Query Engine.

In particular, the pipeline steps are currently the following:

- Define `db`, a class instance wrapper around the `mysql2/promise` Node.js driver for MySQL
- Define `nodejsFnCtx`, an object exposing (a)sync "Queryable" functions that can be safely passed to Rust, so that it can interact with `db`'s class methods
- Load the *debug* version of `libquery`, i.e., the compilation artifact of the `query-engine-node-api` crate
- Define `engine` via the `QueryEngine` constructor exposed by Rust
- Initialize the connector via `engine.connect()`
- Run a Prisma `findMany` query via the JSON protocol, according to the Prisma schema in [./prisma/schema.prisma](./prisma/schema.prisma), storing the result in `resultSet`
- Release the connector via `engine.disconnect()`
- Attempt a reconnection (useful to catch possible panics in the implementation)
- Close the database connection via `nodejsFnCtx`

Example test output:

```
❯ pnpm dev

> @prisma/nodejs-playground@1.0.0 dev /Users/jkomyno/work/prisma/prisma-engines-2/query-engine/nodejs-drivers/nodejs-examples
> ts-node ./src/index.ts

[nodejs] initializing mock connection pool: mysql://root:prisma@localhost:3307/test
[nodejs] initializing mysql connection pool: mysql://root:prisma@localhost:3307/test
fn_ctx: true
QueryEngine {}
[nodejs] connecting...
[nodejs] connected
NodeJSQueryable::query()
NodeJSQueryable::query_raw(SELECT `test`.`some_user`.`id`, `test`.`some_user`.`firstname`, `test`.`some_user`.`company_id` FROM `test`.`some_user` WHERE 1=1, [])
[rs] calling query_raw: SELECT `test`.`some_user`.`id`, `test`.`some_user`.`firstname`, `test`.`some_user`.`company_id` FROM `test`.`some_user` WHERE 1=1
[nodejs] calling queryRaw SELECT `test`.`some_user`.`id`, `test`.`some_user`.`firstname`, `test`.`some_user`.`company_id` FROM `test`.`some_user` WHERE 1=1
[rs] awaiting promise
[nodejs] after query
[nodejs] resultSet {
  columns: [ 'id', 'firstname', 'company_id' ],
  rows: [ [ '1', 'Alberto', '1' ], [ '2', 'Tom', '1' ] ]
}
[rs] awaited: ResultSet { columns: ["id", "firstname", "company_id"], rows: [["1", "Alberto", "1"], ["2", "Tom", "1"]] }
[nodejs] resultSet {"data":{"findManysome_user":[{"id":1,"firstname":"Alberto","company_id":1},{"id":2,"firstname":"Tom","company_id":1}]}}
[nodejs] disconnecting...
[nodejs] disconnected
[nodejs] connecting...
[nodejs] connecting
[nodejs] disconnecting...
[nodejs] disconnected
[nodejs] closing database connection...
[nodejs] calling close() on connection pool
[nodejs] closed connection pool
[nodejs] closed database connection
```

Note how the `[nodejs]` prefixes in the logs arise from using `console.log()` in Node.js, whereas the `[rs]` prefixes arise from using `println!()` in Rust.

Feel free to experiment with different types of queries.

## Main known limitations

- Query parameters are not supported
- Row values from e.g. `findMany` are always cast to string
