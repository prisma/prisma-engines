# Prisma JS Connectors

This TypeScript monorepo contains the following packages:
- `@jkomyno/prisma-js-connector-utils` (later: `@prisma/js-connector-utils`)
  - Internal set of utilities and types for Prisma's JS Connectors.
- `@jkomyno/prisma-neon-js-connector` (later: `@prisma/neon-js-connector`)
  - Prisma's JS Connector that wraps the `@neondatabase/serverless` driver
  - Exposes debug logs via `DEBUG="prisma:js-connector:neon"`
- `@jkomyno/prisma-planetscale-js-connector` (later: `@prisma/planetscale-js-connector`)
  - Prisma's JS Connector that wraps the `@planetscale/database` driver
  - Exposes debug logs via `DEBUG="prisma:js-connector:planetscale"`

## Get Started

We assume Node.js `v18.16.1`+ is installed. If not, run `nvm use` in the current directory.
This is very important to double-check if you have multiple versions installed, as PlanetScale requires either Node.js `v18.16.1`+ or a custom `fetch` function.

Install `pnpm` via:

```sh
npm i -g pnpm
```

## Development

- Install Node.js dependencies via `pnpm i`
- Build and link TypeScript packages via `pnpm build`
- Publish packages to `npm` via `pnpm publish -r`
