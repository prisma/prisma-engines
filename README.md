# Prisma Engines

[![Schema Engine + sql_schema_describer](https://github.com/prisma/prisma-engines/actions/workflows/test-schema-engine.yml/badge.svg)](https://github.com/prisma/prisma-engines/actions/workflows/test-schema-engine.yml)
[![Cargo docs](https://github.com/prisma/prisma-engines/actions/workflows/on-push-to-main.yml/badge.svg)](https://github.com/prisma/prisma-engines/actions/workflows/on-push-to-main.yml)

This repository contains a collection of engines that power the core stack for
[Prisma](https://github.com/prisma/prisma), most prominently [Prisma
Client](https://www.prisma.io/client) and [Prisma
Migrate](https://www.prisma.io/migrate).

If you're looking for how to install Prisma or any of the engines, the [Getting
Started](https://www.prisma.io/docs/getting-started) guide might be useful.

This document describes some of the internals of the engines, and how to build
and test them.

## What's in this repository

This repository contains the following core components:

- _Query compiler_ – compiles Prisma Client queries into executable plans (SQL + orchestration)
  that the client runs through driver adapters in JavaScript land.
- _Driver adapters & executor harness_ – TypeScript utilities that load query plans, talk to
  database drivers, and expose the legacy protocol for existing tooling.
- _Schema engine_ – creates and runs migrations, and performs introspection.
- _Prisma Format_ – historically a formatter for Prisma schemas, now also serves LSP features.

Additionally, the _psl_ (Prisma Schema Language) is the library that defines how the language looks,
how it's parsed, etc.

You'll also find:

- _libs_, for various (small) libraries such as macros, user facing errors,
  various connector/database-specific libraries, etc.
- a `docker-compose.yml` file that's helpful for running tests and bringing up
  containers for various databases
- a `shell.nix` file for bringing up all dependencies and making it easy to
  build the code in this repository (the use of this file and `nix` is
  entirely optional, but can be a good and easy way to get started)
- an `.envrc` file to make it easier to set everything up, including the `nix
shell`

## Documentation

The [API docs (cargo doc)](https://prisma.github.io/prisma-engines/) are
published on our fabulous repo page.

## Building Prisma Engines

**Prerequisites:**

- Installed the latest stable version of the Rust toolchain. You can get the
  toolchain at [rustup](https://rustup.rs/) or the package manager of your
  choice.
- Linux only: OpenSSL is required to be installed.
- Installed [direnv](https://github.com/direnv/direnv), then `direnv allow` on
  the repository root.
  - Make sure direnv is [hooked](https://direnv.net/docs/hook.html) into your shell
  - Alternatively: Load the defined environment in `./.envrc` manually in your
    shell.

Note for nix users: it should be enough to `direnv allow`.
**How to build:**

To build all engines, simply execute `cargo build` on the repository root. This
builds non-production debug binaries. If you want to build the optimized
binaries in release mode, the command is `cargo build --release`.

Depending on how you invoked `cargo` in the previous step, you can find the compiled binaries inside
the repository root in the `target/debug` (without `--release`) or `target/release` directories (with
`--release`):

| Prisma Component | Path to Binary                            |
| ---------------- | ----------------------------------------- |
| Schema Engine    | `./target/[debug\|release]/schema-engine` |
| Prisma Format    | `./target/[debug\|release]/prisma-fmt`    |

The query compiler is a library crate. To produce the Wasm bundles that power the JS runtime, use
`make build-qc-wasm`. Driver adapters are compiled via `make build-driver-adapters-kit-qc`.

## Prisma Schema Language

The _Prisma Schema Language_ is a library which defines the data structures and
parsing rules for prisma files, including the available database connectors. For
more technical details, please check the [library README](./psl/README.md).

The PSL is used throughout the schema engine, as well as prisma format. The DataModel (DML), which
is an annotated version of the PSL, is also used as input for the query compiler and driver adapters.

## Query Compiler & Driver Adapters

Prisma Client now executes queries through the query compiler and TypeScript driver adapters:

- The Rust query compiler consumes the DML and produces query plans describing the SQL and
  orchestration steps required to satisfy a Prisma query.
- Driver adapters (see `libs/driver-adapters`) wrap database drivers in JavaScript. They are
  used by the `@prisma/client-engine-runtime` package in the main repo which implements the
  query plan interpreter and transaction management. Driver adapters are also used directly
  from Rust by the early stage work-in-progress Wasm port of the schema engine.
- The connector test kit (`query-engine/connector-test-kit-rs`) exercises this end-to-end by spawning the
  executor process and driving requests through the adapters.

You will typically touch three layers when working on the query stack:

- Rust planner logic (`query-compiler`, `query-core`, `query-structure`, etc.).
- The driver adapter executor (`libs/driver-adapters/executor`).
- Integration tests (`cargo test -p query-engine-tests`, usually via `make dev-*-qc`).

There is no standalone query engine binary anymore. The compatibility harness lives in JavaScript and is
bundled from this repository using `make build-driver-adapters-kit-qc`.

## Schema Engine

The _Schema Engine_ does a couple of things:

- creates new migrations by comparing the prisma file with the current state of
  the database, in order to bring the database in sync with the prisma file
- run these migrations and keeps track of which migrations have been executed
- (re-)generate a prisma schema file starting from a live database

The engine uses:

- the prisma files, as the source of truth
- the database it connects to, for diffing and running migrations, as well as
  keeping track of migrations in the `_prisma_migrations` table
- the `prisma/migrations` directory which acts as a database of existing
  migrations

## Prisma format

Prisma format can format prisma schema files. It also comes as a Wasm module via
a node package. You can read more [here](./prisma-schema-wasm/README.md).

## Debugging

When trying to debug code, here's a few things that might be useful:

- use the language server; being able to go to definition and reason about code
  can make things a lot easier,
- add `dbg!()` statements to validate code paths, inspect variables, etc.,
- you can control the amount of logs you see, and where they come from using the
  `RUST_LOG` environment variable; see [the documentation](https://docs.rs/env_logger/0.9.1/env_logger/#enabling-logging)

## Testing

There are two test suites for the engines: Unit tests and
integration tests.

- **Unit tests**: They test internal
  functionality of individual crates and components.

  You can find them across the whole codebase, usually in `./tests` folders at
  the root of modules. These tests can be executed via `cargo test`. Note that
  some of them will require the `TEST_DATABASE_URL` enviornment variable set up.

- **Integration tests**: They run GraphQL/JSON requests through the driver adapter executor
  (wrapping the query compiler) and assert that the responses match expectations.

  You can find them at `./query-engine/connector-test-kit-rs`.

> [!NOTE]
> Help needed: document how to run
> - `quaint` tests
> - schema engine tests
>
> This can be a good first contribution.

### Run unit tests

To run unit tests for the whole workspace (except crates that require external services such as
`quaint`, `sql-migration-tests`, or the connector test kit), use:

```bash
make test-unit
```

This target wires up the appropriate `--exclude` list. If you prefer plain cargo, replicate the
exclusions used in the Makefile when invoking `cargo test --workspace --all-features`.

### Set up & run connector test kit (QC)

**Prerequisites:**

- Rust toolchain
- Docker (for SQL connectors)
- Node.js ≥ 20 and pnpm (driver adapters)
- `direnv allow` in the repository root, or load `.envrc` manually

**Setup:**

Use the `dev-*-qc` helpers to spin up a database (when needed), build the query-compiler Wasm, build
the driver adapters, and write the `.test_config` consumed by the connector test kit:

- `make dev-pg-qc`
- `make dev-pg-cockroachdb-qc`
- `make dev-mssql-qc`
- `make dev-planetscale-qc`
- `make dev-mariadb-qc`
- `make dev-libsql-qc`
- `make dev-better-sqlite3-qc`
- `make dev-d1-qc`
- `make dev-neon-qc`

The non-`*-qc` helpers (e.g. `make dev-postgres13`) are still available when you only need a database,
but they do not build driver adapters for you.

_On Windows without WSL:_ replicate what the Make targets do manually (start the container, run
`make build-qc-wasm`, run `make build-driver-adapters-kit-qc`, and create `.test_config`).

**Run:**

```bash
cargo test -p query-engine-tests -- --nocapture
```

Set `DRIVER_ADAPTER=<adapter>` when invoking `make test-qe` to run against a specific adapter, e.g.:

```bash
DRIVER_ADAPTER=pg make test-qe
```

Refer to the [connector test kit guide](./query-engine/connector-test-kit-rs/README.md) for the full
list of adapters, environment variables, and troubleshooting notes.

### Testing driver adapters

Please refer to the [Testing driver adapters](./query-engine/connector-test-kit-rs/README.md)
section in the connector-test-kit-rs README.

**ℹ️ Important note on developing features that require changes to both the query compiler and driver adapter code**

`make test-qe` (optionally with `DRIVER_ADAPTER=...`) ensures you have `prisma/prisma` checked out
next to this repository. The driver adapter sources are symlinked from there so that engines and
client stay in lockstep.

When working on a feature or bugfix spanning adapters and query-compiler code, you will need sibling
PRs in `prisma/prisma` and `prisma/prisma-engines`. Locally, each time you run
`DRIVER_ADAPTER=$adapter make test-qe`, tests use the adapters built from your local `../prisma`
clone.

In CI we need to denote which branch of `prisma/prisma` should be consumed. By default CI clones the
`main` branch, which will not include your local adapter changes. To test in integration, add the
following tag to your PR description on a separate line:

```
/prisma-branch your/branch
```

Replace `your/branch` with the name of your branch in the `prisma` repository.

GitHub actions will then pick up the branch name and use it to clone that branch's code of prisma/prisma, and build the driver adapters code from there.

When it's time to merge the sibling PRs, you'll need to merge the prisma/prisma PR first, so when merging the engines PR you have the code of the adapters ready in prisma/prisma `main` branch.

### Testing engines in `prisma/prisma`

You can trigger releases from this repository to npm that can be used for testing the engines in `prisma/prisma` either automatically or manually:

#### Automated integration releases from this repository to npm

Any branch name starting with `integration/` will, first, run the full test suite in GH Actions and, second, run the release workflow (build and upload engines to S3 & R2).
To trigger the release on any other branch, you have two options:

- Either run [build-engines](https://github.com/prisma/prisma-engines/actions/workflows/build-engines.yml) workflow on a specified branch manually.
- Or add `[integration]` string anywhere in your commit messages/

The journey through the pipeline is the same as a commit on the `main` branch.

- It will trigger [`prisma/engines-wrapper`](https://github.com/prisma/engines-wrapper) and publish a new [`@prisma/engines-version`](https://www.npmjs.com/package/@prisma/engines-version) npm package but on the `integration` tag.
- Which triggers [`prisma/prisma`](https://github.com/prisma/prisma) to create a `chore(Automated Integration PR): [...]` PR with a branch name also starting with `integration/`
- Since in `prisma/prisma` we also trigger the publish pipeline when a branch name starts with `integration/`, this will publish all `prisma/prisma` monorepo packages to npm on the `integration` tag.
- Our [ecosystem-tests](https://github.com/prisma/ecosystem-tests/) tests will automatically pick up this new version and run tests, results will show in [GitHub Actions](https://github.com/prisma/ecosystem-tests/actions?query=branch%3Aintegration)

This end to end will take minimum ~1h20 to complete, but is completely automated :robot:

Notes:

- tests and publishing workflows are run in parallel in both `prisma/prisma-engines` and `prisma/prisma` repositories. So, it is possible that the engines would be published and only then test suite will
  discover a defect. It is advised that to keep an eye on both test and publishing workflows.

#### Manual integration releases from this repository to npm

Additionally to the automated integration release for `integration/` branches, you can also trigger a publish by pushing a commit with the content `[integration]`.

## Parallel rust-analyzer builds

When rust-analzyer runs `cargo check` it will lock the build directory and stop any cargo commands from running until it has completed. This makes the build process feel a lot longer. It is possible to avoid this by setting a different build path for
rust-analyzer. To avoid this. Open VSCode settings and search for `Check on Save: Extra Args`. Look for the `Rust-analyzer › Check On Save: Extra Args` settings and add a new directory for rust-analyzer. Something like:

```
--target-dir:/tmp/rust-analyzer-check
```

## Community PRs: create a local branch for a branch coming from a fork

To trigger an [Automated integration releases from this repository to npm](#automated-integration-releases-from-this-repository-to-npm) or [Manual integration releases from this repository to npm](#manual-integration-releases-from-this-repository-to-npm) branches of forks need to be pulled into this repository so the Github Actions job is triggered. You can use these GitHub and git CLI commands to achieve that easily:

```
gh pr checkout 4375
git checkout -b integration/sql-nested-transactions
git push --set-upstream origin integration/sql-nested-transactions
```

If there is a need to re-create this branch because it has been updated, deleting it and re-creating will make sure the content is identical and avoid any conflicts.

```
git branch --delete integration/sql-nested-transactions
gh pr checkout 4375
git checkout -b integration/sql-nested-transactions
git push --set-upstream origin integration/sql-nested-transactions --force
```

## Security

If you have a security issue to report, please contact us at [security@prisma.io](mailto:security@prisma.io?subject=[GitHub]%20Prisma%202%20Security%20Report%20Engines)
