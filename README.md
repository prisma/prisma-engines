# Quaint
[![Build status](https://badge.buildkite.com/c30bc2b4dccc155aec44608ad5f366feabdab121295ceb6b6b.svg)](https://buildkite.com/prisma/quaint)

Quaint is an abstraction over certain SQL databases. It provides:

- An AST for building dynamic SQL queries.
- Visitors for different databases to generate SQL strings.
- Connectors to abstract over results and querying.
- Pooling with [mobc](https://crates.io/crates/mobc)
- Async/await and Futures 0.3

### Documentation

- [Released](https://docs.rs/quaint)
- [Master](https://prisma.github.io/quaint/quaint/index.html)

### Feature flags

- `full`: All connectors and a pooled `Quaint` manager
- `full-postgresql`: Pooled support for PostgreSQL
- `full-mysql`: Pooled support for MySQL
- `full-sqlite`: Pooled support for SQLite
- `single`: All connectors, but no pooling
- `single-postgresql`: Single connection support for PostgreSQL
- `single-mysql`: Single connection support for MySQL
- `single-sqlite`: Single connection support for SQLite

### Goals:

- Query generation when the database and conditions are not known beforehand.
- Parameterized queries and SQL injection protection.
- A modular design, a separate AST and separate visitors and connectors.

### Non-goals:

- Database-level type-safety in query building or being an ORM.

For type-safe database abstraction, [Diesel](https://diesel.rs/) is an excellent
choice.

### Testing:

- See `.envrc` for connection params. Override variables if different. MySQL and
  PostgreSQL needs to be running for tests to succeed.
  
Then:
  
``` sh
> cargo test
```
