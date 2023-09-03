# @prisma/smoke-test-js

This is a playground for testing the `libquery` client with the experimental Node.js drivers.
It contains a subset of `@prisma/client`, plus some handy executable smoke tests:
- [`./src/planetscale.ts`](./src/planetscale.ts)
- [`./src/neon.ts`](./src/neon.ts)
- [`./src/libsql.ts`](./src/libsql.ts)

## How to setup

We assume Node.js `v18.16.1`+ is installed. If not, run `nvm use` in the current directory.
This is very important to double-check if you have multiple versions installed, as PlanetScale requires either Node.js `v18.16.1`+ or a custom `fetch` function.

- Create a `.envrc` starting from `.envrc.example`, and fill in the missing values following the given template
- Install Node.js dependencies via
  ```bash
  pnpm i
  ```
- Run `cargo build -p query-engine-node-api` to compile the `libquery` Query Engine
- Build the JS Connectors: `cd .. && pnpm run -r build`

### PlanetScale

- Create a new database on [PlanetScale](https://planetscale.com/)
- Go to `Settings` > `Passwords`, and create a new password for the `main` database branch. Select the `Prisma` template and copy the generated URL (comprising username, password, etc). Paste it in the `JS_PLANETSCALE_DATABASE_URL` environment variable in `.envrc`.

In the current directory:
- Run `pnpm prisma:planetscale` to push the Prisma schema and insert the test data.
- Run `pnpm planetscale` to run smoke tests against the PlanetScale database.

### Neon

- Create a new database with Neon CLI `npx neonctl projects create` or in [Neon Console](https://neon.tech).
- Paste the connection string to `JS_NEON_DATABASE_URL`. 

In the current directory:
- Run `pnpm prisma:neon` to push the Prisma schema and insert the test data.
- Run `pnpm neon` to run smoke tests against the Neon database.

### libsql/Turso

- Create a new database with [Turso CLI](https://docs.turso.tech/reference/libsql-cli) `turso db create` and then get connection string and authentication token: `turso db show ...` + `turso db tokens create ...`
- Store both in `JS_LIBSQL_DATABASE_URL` and `JS_LIBSQL_TOKEN`. 

In the current directory:
- ~~Run `pnpm prisma:libsql` to push the Prisma schema and insert the test data.~~
  - Manually run `migrations/20230903101652_init/migration.sql` and `commands/type_test/insert.sql` via `turso db shell`
    - Migration was originally created via `pnpm prisma migrate dev --schema ./prisma/sqlite-libsql/schema.prisma`
- Run `pnpm libsql` to run smoke tests against the LibSQL/Turso database.
