# Quaint
[![Build status](https://badge.buildkite.com/c30bc2b4dccc155aec44608ad5f366feabdab121295ceb6b6b.svg)](https://buildkite.com/prisma/quaint)

Quaint is an abstraction over certain SQL databases. It provides:

- An AST for building dynamic SQL queries.
- Visitors for different databases to generate SQL strings.
- Connectors to abstract over results and querying.
- Pooling with [tokio-resource-pool](https://crates.io/crates/tokio-resource-pool)

Example:

``` rust
use quaint::{ast::*, Quaint};

#[tokio::main]
async fn main() -> Result<(), quaint::error::Error> {
    let quaint = Quaint::new("postgres://user:pass@localhost/mydb")?;
    let conn = quaint.check_out().await?;

    let query = Select::from_table("cats").so_that("name".equals("musti"));
    let result = conn.select(query).await?;

    assert_eq!(
        Some(1),
        result.into_iter().nth(0).and_then(|row| row[0].as_i64()),
    );

    Ok(())
}
```

### Documentation

- [Master](https://prisma.github.io/quaint/quaint/index.html)
- [Released](https://docs.rs/quaint)

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
