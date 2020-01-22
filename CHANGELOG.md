# Changelog

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
