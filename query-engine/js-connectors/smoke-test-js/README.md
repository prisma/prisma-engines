# @prisma/smoke-test-js

This is a playground for testing the `libquery` client with the experimental Node.js drivers.
It contains a subset of `@prisma/client`, plus some handy executable smoke tests:
- [`./src/planetscale.ts`](./src/planetscale.ts)
- [`./src/neon.ts`](./src/neon.ts)

## How to setup

We assume Node.js `v18.16.1`+ is installed.

- Create a `.envrc` starting from `.envrc.example`, and fill in the missing values following the given template
- Install Node.js dependencies via
  ```bash
  pnpm i
  ```

### PlanetScale

- Create a new database on [PlanetScale](https://planetscale.com/)
- Go to `Settings` > `Passwords`, and create a new password for the `main` database branch. Select the `Prisma` template and copy the generated URL (comprising username, password, etc). Paste it in the `JS_PLANETSCALE_DATABASE_URL` environment variable in `.envrc`.
- Create a new `shadow` database branch. Repeat the steps above (selecting the `shadow` branch instead of `main`), and paste the generated URL in the `JS_PLANETSCALE_SHADOW_DATABASE_URL` environment variable in `.envrc`.

In the current directory:
- Set the provider in [./prisma/mysql-planetscale/schema.prisma](./prisma/mysql-planetscale/schema.prisma) to `mysql`.
- Run `npx prisma db push --schema ./prisma/mysql-planetscale/schema.prisma`
- Run `npx prisma migrate deploy --schema ./prisma/mysql-planetscale/schema.prisma`
- Set the provider in [./prisma/mysql-planetscale/schema.prisma](./prisma/mysql-planetscale/schema.prisma) to `@prisma/planetscale`.

Note: you used to be able to run these Prisma commands without changing the provider name, but [#4074](https://github.com/prisma/prisma-engines/pull/4074) changed that (see https://github.com/prisma/prisma-engines/pull/4074#issuecomment-1649942475).

### Neon

- Create a new database with Neon CLI `npx neonctl projects create` or in [Neon Console](https://neon.tech).
- Paste the connection string to `JS_NEON_DATABASE_URL`. Create a shadow branch and repeat the step above, paste the connection string to `JS_NEON_SHADOW_DATABASE_URL`.

In the current directory:
- Set the provider in [./prisma/postgres-neon/schema.prisma](./prisma/postgres-neon/schema.prisma) to `postgres`.
- Run `npx prisma db push --schema ./prisma/postgres-neon/schema.prisma`
- Run `npx prisma migrate deploy --schema ./prisma/postgres-neon/schema.prisma`
- Set the provider in [./prisma/postgres-neon/schema.prisma](./prisma/postgres-neon/schema.prisma) to `@prisma/neon`.

## How to use

In the current directory:
- Run `cargo build -p query-engine-node-api` to compile the `libquery` Query Engine
- Run `pnpm planetscale` to run smoke tests against the PlanetScale database
- Run `pnpm neon` to run smoke tests against the PlanetScale database

## How to test

There is no automatic test. However, [./src/index.ts](./src/index.ts) includes a pipeline you can use to interactively experiment with the new Query Engine.

In particular, the pipeline steps are currently the following (in the case of PlanetScale):

- Define `db`, a class instance wrapper around the `@planetscale/database` JS driver for PlanetScale
- Define `nodejsFnCtx`, an object exposing (a)sync "Queryable" functions that can be safely passed to Rust, so that it can interact with `db`'s class methods
- Load the *debug* version of `libquery`, i.e., the compilation artifact of the `query-engine-node-api` crate
- Define `engine` via the `QueryEngine` constructor exposed by Rust
- Initialize the connector via `engine.connect()`
- Run a Prisma `findMany` query via the JSON protocol, according to the Prisma schema in [./prisma/mysql-planetscale/schema.prisma](./prisma/mysql-planetscale/schema.prisma), storing the result in `resultSet`
- Release the connector via `engine.disconnect()`
- Attempt a reconnection (useful to catch possible panics in the implementation)
- Close the database connection via `nodejsFnCtx`

Example test output:

```
❯ npm run planetscale

> @prisma/jsdrivers-playground@1.0.0 planetscale
> ts-node ./src/planetscale.ts

[nodejs] connecting...
[nodejs] connected
[nodejs] isHealthy false
[nodejs] findMany resultSet {
  "data": {
    "findManytype_test": [
      {
        "tinyint_column": 127,
        "smallint_column": 32767,
        "mediumint_column": 8388607,
        "int_column": 2147483647,
        "bigint_column": {
          "$type": "BigInt",
          "value": "9223372036854775807"
        },
        "float_column": 3.4,
        "double_column": 1.7977,
        "decimal_column": {
          "$type": "Decimal",
          "value": "99999999.99"
        },
        "boolean_column": true,
        "char_column": "c",
        "varchar_column": "Sample varchar",
        "text_column": "This is a long text...",
        "date_column": {
          "$type": "DateTime",
          "value": "2023-07-24T00:00:00.000Z"
        },
        "time_column": {
          "$type": "DateTime",
          "value": "1970-01-01T23:59:59.000Z"
        },
        "datetime_column": {
          "$type": "DateTime",
          "value": "2023-07-24T23:59:59.000Z"
        },
        "timestamp_column": {
          "$type": "DateTime",
          "value": "2023-07-24T23:59:59.000Z"
        },
        "json_column": {
          "$type": "Json",
          "value": "{\"key\":\"value\"}"
        },
        "enum_column": "value3",
        "binary_column": {
          "$type": "Bytes",
          "value": "TXlTUUwAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=="
        },
        "varbinary_column": {
          "$type": "Bytes",
          "value": "SGVsbG8g"
        },
        "blob_column": {
          "$type": "Bytes",
          "value": "YmluYXJ5"
        }
      }
    ]
  }
}
[nodejs] disconnecting...
[nodejs] disconnected
[nodejs] re-connecting...
[nodejs] re-connecting
[nodejs] re-disconnecting...
[nodejs] re-disconnected
[nodejs] closing database connection...
[nodejs] closed database connection
```
