# PSL

## Tests

### Running the tests

To run the PSL test suite, run `cargo test` from this directory.

### Contributing tests

Many tests are plain cargo test functions using the `#[test]` attribute.

There are also declarative test suites that do not involve writing Rust code:

- **validation tests** are located in the `tests/validation` directory. Each test is a single Prisma schema. The comment at the end of the file is a snapshot of the validation errors that PSL would return upon validating the schema.
- **reformatter tests** are located in the `tests/reformatter` directory. Each test is a Prisma schema. The test suite tests its reformatted version against a snapshot that is a file next to it, with the same name, except that it has the `.reformatted.prisma` extension instead of `.prisma`.

These tests are transformed into Rust code via the build script in `build.rs`.
