# @prisma/smoke-test-wasm

This is a playground for testing the `libquery` client with the experimental Node.js drivers.
It contains a subset of `@prisma/client`, plus some handy executable smoke tests:
- [`./src/neon.ts`](./src/neon.ts)

## How to setup

We assume Node.js `v18.16.1`+ is installed. If not, run `nvm use` in the current directory.
This is very important to double-check if you have multiple versions installed, as PlanetScale requires either Node.js `v18.16.1`+ or a custom `fetch` function.

- Create a `.envrc` starting from `.envrc.example`, and fill in the missing values following the given template
- Install Node.js dependencies via
  ```bash
  pnpm i
  ```
- Run `wasm-pack build ../../../query-engine-wasm-api --target web` to compile the `libquery` Query Engine

### Neon

- Create a new database with Neon CLI `npx neonctl projects create` or in [Neon Console](https://neon.tech).
- Paste the connection string to `JS_NEON_DATABASE_URL`. 

In the current directory:
- Run `pnpm prisma:neon` to push the Prisma schema and insert the test data.
- Run `pnpm neon` to run smoke tests against the Neon database.
