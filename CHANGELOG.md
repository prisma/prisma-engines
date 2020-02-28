# Changelog

## v0.2.0-alpha.9

- Correct position for GROUP BY if having ORDER BY in the same clause

## v0.2.0-alpha.8

- AVG and SUM implementations

## v0.2.0-alpha.7

- Fix a missing dependency when compiling using only single-sqlite feature

## v0.2.0-alpha.6

- Remove lazy_static in favor of once_cell

## v0.2.0-alpha.5

- Fix possible stack overflows with conditions
- Foreign key constraint errors

## v0.2.0-alpha.4

- Fix a deadlock in sqlite when panicking
- Introduce VALUES construct

## v0.2.0-alpha.3

- Fix broken less_than in a row

## v0.2.0-alpha.2

- Implement support for Arrays of Enums

## v0.2.0-alpha.1

Breaking changes ahead

- Errors redesign. (https://github.com/prisma/quaint/pull/72)
- Queryable redesign. (https://github.com/prisma/quaint/pull/61) and (https://github.com/prisma/quaint/pull/74)
- Unique/null constr error should be multi-column (https://github.com/prisma/quaint/pull/62)
- Add optional serde support (https://github.com/prisma/quaint/pull/63)

## v0.1.13

- Correct position for GROUP BY if having ORDER BY in the same clause

## v0.1.12

- Add a ParameterizedValue::Enum in order to support writing to native enum columns in Postgres

## v0.1.11

- Update mobc to 0.5, do not check connection status on check-out

## v0.1.10

- Make socket timeouts optional

## v0.1.9

- Internal fix for faster result row generation (#65)
- Make Postgres initialization to not use prepared statements, making it
  possible to use it in pgbouncer transactional mode (#67)

## v0.1.8

- Adding timeout configuration (https://github.com/prisma/quaint/pull/66)

## v0.1.7

- Fixing clippy warnings, 2020 edition
- Add is_* methods to SqlFamily
- Add item type to the tracing query log

## v0.1.6

- Loosen up certain vector-taking functions to use `IntoIter`
- Fix the `columns` method to actually use `Column` instead of a plain `DatabaseValue`

## v0.1.5

- `Quaint` to implement `Clone`

## v0.1.4

- Update to UUID 0.8 and replace the feature flag with the right version

## v0.1.3

- More documentation

## v0.1.2

- Set docs.rs to build docs with the `full` feature flag

## v0.1.1

- Error enum implements std::error::Error
- Docker image fixes
- Crates.io badge

## v0.1.0

- Initial relese
