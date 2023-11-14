# Quaint

[![crates.io](https://meritbadge.herokuapp.com/quaint)](https://crates.io/crates/quaint)
[![docs.rs](https://docs.rs/quaint/badge.svg)](https://docs.rs/quaint)
[![Cargo tests](https://github.com/prisma/quaint/actions/workflows/test.yml/badge.svg)](https://github.com/prisma/quaint/actions/workflows/test.yml)
[![Discord](https://img.shields.io/discord/664092374359605268)](https://discord.gg/r4CPY4B)

Quaint is an abstraction over certain SQL databases. It provides:

- An AST for building dynamic SQL queries.
- Visitors for different databases to generate SQL strings.
- Connectors to abstract over results and querying.
- Pooling with [mobc](https://crates.io/crates/mobc)
- Async/await and Futures 0.3

### Feature flags

- `mysql`: Support for MySQL databases.
  - On non-WebAssembly targets, choose `mysql-native` instead.
- `postgresql`: Support for PostgreSQL databases.
  - On non-WebAssembly targets, choose `postgresql-native` instead.
- `sqlite`: Support for SQLite databases.
  - On non-WebAssembly targets, choose `sqlite-native` instead.
- `mssql`: Support for Microsoft SQL Server databases.
  - On non-WebAssembly targets, choose `mssql-native` instead.
- `pooled`: A connection pool in `pooled::Quaint`.
- `vendored-openssl`: Statically links against a vendored OpenSSL library on
  non-Windows or non-Apple platforms.
- `fmt-sql`: Enables logging SQL queries _formatted_. The `FMT_SQL` env var must be present for the formatting to be enabled.

### Goals:

- Query generation when the database and conditions are not known beforehand.
- Parameterized queries and SQL injection protection.
- A modular design, a separate AST and separate visitors and connectors.

### Non-goals:

- Database-level type-safety in query building or being an ORM.

For type-safe database abstraction, [Diesel](https://diesel.rs/) is an excellent
choice.

### Building

```sh
 > cargo build --features all
```

### Testing

- See `.envrc` for connection params. Override variables if different. MySQL,
  PostgreSQL and SQL Server needs to be running for tests to succeed.

Then:

```sh
> cargo test
```

### Query debug

The queries can be logged by setting the `LOG_QUERIES` environment variable to any
value. They'll be logged at the `INFO` level and are visible when having a
logger in scope.

The `FMT_SQL` environment variable can be used to log _formatted_ SQL queries. Beware, the `fmt-sql` feature must be enabled too.

### Generating docs

This requires the rust nightly channel:

```sh
> cargo rustdoc --all-features
```

Documentation index would be created at `$CARGO_TARGET_DIR/doc/quaint/index.html`.

## Security

If you have a security issue to report, please contact us at [security@prisma.io](mailto:security@prisma.io?subject=[GitHub]%20Prisma%202%20Security%20Report%20Quaint).
