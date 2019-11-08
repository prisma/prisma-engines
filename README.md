# Prisma Engines

The core stack for [Photon](https://github.com/prisma/photonjs/) and
[Lift](https://github.com/prisma/lift/).

## Code architecture

Photon uses query-engine and its
[main](https://github.com/prisma/prisma-engine/blob/master/query-engine/prisma/src/main.rs)
is a good place to start digging into the code.

The request basically flows from
[server.rs](https://github.com/prisma/prisma-engine/blob/master/query-engine/prisma/src/server.rs)
to [graphql
handler](https://github.com/prisma/prisma-engine/blob/master/query-engine/prisma/src/request_handlers/graphql/handler.rs)
and from there to [core
executor](https://github.com/prisma/prisma-engine/blob/master/query-engine/core/src/executor/interpreting_executor.rs)
down to the
[connectors](https://github.com/prisma/prisma-engine/tree/master/query-engine/connectors/sql-query-connector/src).

The SQL generation and SQL database abstractions are handled by the [quaint
crate](https://github.com/prisma/quaint).

Lift connects to the migration-engine and the starting point for requests is the
[rpc
api](https://github.com/prisma/prisma-engine/blob/master/migration-engine/core/src/api/rpc.rs).

This section contains instructions for building the binaries that are powering the [Prisma Framework](https://github.com/prisma/prisma2) with a [**development**](https://doc.rust-lang.org/book/ch14-01-release-profiles.html) release profile (i.e. in _debug mode_).

## Building Binaries in Debug Mode

### 1. Clone the repository

First, you need to clone this repository and navigate into its root folder:

```
git clone git@github.com:prisma/prisma-engine.git
cd prisma-engine
```

### 2. Switch to the beta version of Rust

You can switch to Rust's beta version using the following command:

```
rustup default beta
```

Afterwards you can verify that the switch worked by running `rustc --version`. If your version includes `beta`, the switch was successful.

### 3. Build binaries in development mode

The development release profile is the default when you're building your code with [Cargo](https://doc.rust-lang.org/cargo/)'s `build` command. Therefore, you can build your project in debug mode as follows:

```
cargo build
```

### 4. Access the built binaries

You can find the compiled binaries inside the newly created `./target/debug` directory:

| Prisma Framework Component | Path to Binary                                       |
| -------------------------- | ---------------------------------------------------- |
| HTTP server + Query Engine | `./target/prisma/prisma`                             |
| Migration Engine           | `./target/migration-engine/migration-engine`         |
| Introspection Engine       | `./target/introspection-engine/introspection-engine` |
| Prisma Format              | `./target/prisma-fmt/prisma-fmt`                     |

## Coding Guidelines

- Prevent compiler warnings
- Use Rust formatting (`cargo fmt`)

## Testing

- To compile all modules use the provided `build.sh` script
- To test all modules use the provided `test.sh` script
