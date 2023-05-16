# @prisma/nodejs-playground

This is a playground for testing the `libquery` client with the experimental Node.js drivers.
It contains a subset of `@prisma/client`, plus a handy [`index.ts`](./src/index.ts) file with a `main` function.

## How to use

In the root directory:
  - Run `cargo build -p query-engine-node-api --release` to compile the `libquery` Query Engine
  - Spawn a MySQL8 database via `make dev-mysql8`
  - Store the `export TEST_DATABASE_URL="mysql://root:prisma@localhost:3307"` env var in `.envrc.local` and expose it via `direnv allow .`
  - Copy the content of [`./src/index.sql`](./src/index.sql) into the MySQL8 database available at `mysql://root:prisma@localhost:3307`

In the current directory
  - Run `pnpm i` to install dependencies
  - Run `pnpm dev` to run the playground

## How to test

There is no automatic test. However, you could add `println!("[rs] ...: {}", ...)` statements to `query-engine-node-api` in the `QueryEngineNodeDrivers` constructor, to see the values returned by the Node.js functions (read in `fn_ctx`).
