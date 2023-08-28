# @prisma/smoke-test-js

This is a playground for testing the `libquery` client with the experimental Node.js Drivers / Driver Adapters.
It contains a subset of `@prisma/client`, plus some handy executable smoke tests:
- [`./src/neon.ws.ts`](./src/neon.ws.ts)
- [`./src/neon.http.ts`](./src/neon.http.ts)
- [`./src/planetscale.ts`](./src/planetscale.ts)
- [`./src/pg.ts`](./src/pg.ts)

## How to setup

We assume Node.js `v18.16.1`+ is installed. If not, run `nvm use` in the current directory.
This is very important to double-check if you have multiple versions installed, as PlanetScale requires either Node.js `v18.16.1`+ or a custom `fetch` function.

- Create a `.envrc` starting from `.envrc.example`, and fill in the missing values following the given template
- Install Node.js dependencies via
  ```bash
  pnpm i
  ```
- Run `cargo build -p query-engine-node-api` to compile the `libquery` Query Engine

### PlanetScale

- Create a new database on [PlanetScale](https://planetscale.com/)
- Go to `Settings` > `Passwords`, and create a new password for the `main` database branch. Select the `Prisma` template and copy the generated URL (comprising username, password, etc). Paste it in the `JS_PLANETSCALE_DATABASE_URL` environment variable in `.envrc`.

In the current directory:
- Run `pnpm prisma:planetscale` to push the Prisma schema and insert the test data.
- Run `pnpm planetscale` to run smoke tests against the PlanetScale database.

Note: you used to be able to run these Prisma commands without changing the provider name, but [#4074](https://github.com/prisma/prisma-engines/pull/4074) changed that (see https://github.com/prisma/prisma-engines/pull/4074#issuecomment-1649942475).

### Neon

- Create a new database with Neon CLI `npx neonctl projects create` or in [Neon Console](https://neon.tech).
- Paste the connection string to `JS_NEON_DATABASE_URL`. 

In the current directory:
- Run `pnpm prisma:neon` to push the Prisma schema and insert the test data.
- Run `pnpm neon:ws` to run smoke tests against the Neon database using WebSockets under the hood.
- Run `pnpm neon:ws` to run smoke tests against the Neon database using HTTP under the hood. Note: this currently results in errors.

### Postgres

- Create a new Postgres database with [Prisma Provision](https://db-provision.cloud.prisma.io/) or from a local `postgres` Docker instance.
- Paste the connection string to `JS_PG_DATABASE_URL`. Note: connections strings that use a remote host require `?sslmode=disable` to work with the Postgres adapter.

In the current directory:
- Run `pnpm prisma:pg` to push the Prisma schema and insert the test data.
- Run `pnpm pg` to run smoke tests against the Postgres database.
