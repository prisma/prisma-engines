# @prisma/smoke-test-js

This is a playground for testing the `libquery` client with the experimental Node.js drivers.
It contains a subset of `@prisma/client`, plus some handy executable smoke tests:
- [`./src/planetscale.ts`](./src/planetscale.ts)
- [`./src/neon.ts`](./src/neon.ts)
- [`./src/pg.ts`](./src/pg.ts)
- [`./src/xata.ts`](./src/xata.ts)

## How to setup

We assume Node.js `v18.16.1`+ is installed. If not, run `nvm use` in the current directory.
This is very important to double-check if you have multiple versions installed, as PlanetScale requires either Node.js `v18.16.1`+ or a custom `fetch` function.

- Create a `.envrc` starting from `.envrc.example`, and fill in the missing values following the given template
- Install Node.js dependencies via
  ```bash
  pnpm i
  ```
- Run `cargo build -p query-engine-node-api` to compile the `libquery` Query Engine
- Decide if you want to have debug output, and add `DEBUG="*"` to your env if you do.

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
- Run `pnpm neon` to run smoke tests against the Neon database.

### Xata

- Create a new database in the Xata web UI and note the database URL.
- Use Xata CLI to get an API key: `pnpx @xata.io/cli auth login` + `pnpx @xata.io/cli init` -> creates `.env`
- Manually create the tables to match the smoke test schema, then run `pnpx @xata.io/cli pull main` to pull schema changes to regenerate code and types
  - `pnpx @xata.io/cli codegen` to just regenerate the code

In the current directory:
- Run `pnpm xata` to run smoke tests against the Xata database.
