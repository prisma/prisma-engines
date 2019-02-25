# prisma-query

prisma-query is an AST and database-specific visitors for creating SQL
statements.

*Under construction and will go through several rounds of changes*. Not meant
for production use in the current form.

### Goals:

- Query generation when the database and conditions are not known beforehand.
- Parameterized queries when possible.
- A modular design, separate AST and visitor when extending to new databases.
- Database support behind a feature flag.

### Non-goals:

- Database-level type-safety in query building or being an ORM.

## Database priorities:

- SQLite will be the first Visitor
- PostgreSQL
- MySQL

More databases will be decided later.
