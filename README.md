# Prisma Engines

This repository contains a collection of engines that power the core stack for
[Prisma](https://github.com/prisma/prisma2), most prominently [Prisma
Client](https://github.com/prisma/prisma-client-js) and [Prisma
Migrate](https://github.com/prisma/migrate).

The engines and their respective binary crates are:
- Query engine: `prisma`
- Migration engine: `migration-engine`
- Introspection engine: `introspection-engine`
- Prisma Format: `prisma-fmt`

## Building Prisma Engines

**Prerequisites:**
- Installed the stable Rust toolchain, at least version 1.39.0. You can get the
  toolchain at [rustup](https://rustup.rs/) or the package manager of your
  choice.
- Linux only: OpenSSL is required to be installed.
- Installed [direnv](https://github.com/direnv/direnv), then `direnv allow` on
  the repository root.
    - Alternatively: Load the defined environment in `./.envrc` manually in your
      shell.

**How to build:**

To build all engines, simply execute `cargo build` on the repository root. This
builds non-production debug binaries. If you want to build the optimized
binaries in release mode, the command is `cargo build --release`.

Depending on how you invoked `cargo` in the previous step, you can find the
compiled binaries inside the repository root in the `target/debug` (without
`--release`) or `target/release` directories (with `--release`):

| Prisma Component           | Path to Binary                                            |
| -------------------------- | --------------------------------------------------------- |
| Query Engine               | `./target/[debug\|release]/prisma`                         |
| Migration Engine           | `./target/[debug\|release]/migration-engine`               |
| Introspection Engine       | `./target/[debug\|release]/introspection-engine`           |
| Prisma Format              | `./target/[debug\|release]/prisma-fmt`                     |

## Usage

The Query Engine can be run as a graphql server without using the Prisma Client.
If using it on production please be aware the api and the query language can
change any time. There is no guaranteed API stability.

Notable environment flags:
- `RUST_LOG_FORMAT=(devel|json)` sets the log format. By default outputs `json`.
- `PRISMA_DML_PATH=[path_to_datamodel_file]` should point to the datamodel file
  location. This or `PRISMA_DML` is required for the Query Engine to run.
- `PRISMA_DML=[base64_encoded_datamodel]` an alternative way to provide a
  datamodel for the server.
- `RUST_BACKTRACE=(0|1)` if set to 1, the error backtraces will be printed to
  the STDERR.
- `LOG_QUERIES=[anything]` if set, the SQL queries will be written to the `INFO`
  log. Needs the right log level enabled to be seen from the terminal.
- `RUST_LOG=[filter]` sets the filter for the logger. Can be either `trace`,
  `debug`, `info`, `warning` or `error`, that will output ALL logs from every
  crate from that level. The `.envrc` in this repo shows how to log different
  parts of the system in a more granular way.

Starting the Query Engine:

The engine can be started either with using the `cargo` build tool, or
pre-building a binary and running it directly. If using `cargo`, replace
whatever command that starts with `./prisma` with `cargo run --bin prisma --`.

**Help**
```bash
> ./target/release/prisma --help
prisma d6f9915c25a2ae6eb793a3a18f87e576fb82e9da

USAGE:
    prisma [FLAGS] [OPTIONS] [SUBCOMMAND]

FLAGS:
        --always-force-transactions    Runs all queries in a transaction, including all the reads
        --enable-raw-queries           Enables raw SQL queries with executeRaw mutation
    -h, --help                         Prints help information
        --legacy                       Switches query schema generation to Prisma 1 compatible mode
    -V, --version                      Prints version information

OPTIONS:
        --host <host>    The hostname or IP the query engine should bind to [default: 127.0.0.1]
    -p, --port <port>    The port the query engine should bind to [env: PORT=]  [default: 4466]

SUBCOMMANDS:
    cli     Doesn't start a server, but allows running specific commands against Prisma
    help    Prints this message or the help of the given subcommand(s)

> ./target/release/prisma cli --help
Doesn't start a server, but allows running specific commands against Prisma

USAGE:
    prisma cli <SUBCOMMAND>

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

SUBCOMMANDS:
    dmmf               Output the DMMF from the loaded data model
    dmmf-to-dml        Convert the given DMMF JSON file to a data model
    execute-request    Executes one request and then terminates
    get-config         Get the configuration from the given data model
    help                 Prints this message or the help of the given subcommand(s)
```

The prisma version hash is the latest git commit at the time the binary was built.

## Testing

There are two test suites for the engines: Unit tests ("Cargo tests") and
integration tests ("Connector TestKit").

The Unit tests are implemented in the Rust code. They test internal
functionality of individual crates and components.

The Connector TestKit is a separate Scala project found at
`./query-engine/connector-test-kit` that runs GraphQL queries against isolated
instances of the query engine and asserts that the responses are correct.

### Set up & run integration tests:
**Prerequisites:**
- Installed Rust toolchain.
- Installed Docker and Docker-Compose.
- Installed Java, Scala, SBT (Scala Build Tool).
- Installed `direnv`, then `direnv allow` on the repository root.
    - Alternatively: Load the defined environment in `./.envrc` manually in your shell.

**Setup**:
There are helper `make` commands to set up a test environment for a specific
database connector you want to test. The commands set up a container (if needed)
and write the `current_connector` file, which is picked up by the integration
tests:

- `make dev-mysql`: MySQL 5.7
- `make dev-mysql8`: MySQL 8
- `make dev-postgres`: PostgreSQL 10
- `make dev-sqlite`: SQLite

As an optional but recommended step, you can run the tests by setting up an
IntelliJ project for `./query-engine/connector-test-kit`, which makes test
results much more accessible. You need to install the Scala plugin for Intellij
if you want to do so.

**Run:**
If you're using Intellij, you can run all tests by right-clicking
`src/test/scala` > `Run ScalaTests`.

If you want to use the command line, start `sbt` in
`./query-engine/connector-test-kit`, then execute `test` in the sbt shell.

### Set up & run cargo tests:

**Prerequisites:**
- Installed Rust roolchain.
- Installed Docker and Docker-Compose.
- Installed `direnv`, then `direnv allow` on the repository root.
    - Alternatively: Load the defined environment in `./.envrc` manually in your shell.
- Start all test databases with `make all-dbs`.

**Run:**
Run `cargo test -- --test-threads 1` in the repository root.

## WIP Coding Guidelines
- Prevent compiler warnings
- Use Rust formatting (`cargo fmt`)
