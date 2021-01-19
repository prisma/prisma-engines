### GraphQL query

```graphql
{
  findManyItems(
    where: {
      json: {
        path: { value: “a.b” },
        # ...
      } 
    }
  ) {
    id
    json
  }
}
```

### PostgreSQL

```sql
SELECT json FROM column WHERE json#>>'{a, b}'
```

### MySQL

```sql
SELECT json FROM column WHERE JSON_EXTRACT(json, '$.a.b')
```

### Problems

- How to get different function/operators between mysql/postgres?
- How to express an operator?
- How to express a function? (I think I got that one)