# Contributing to datamodel

## Tests

###  Running the test suite

Run `cargo test` for this crate (`datamodel`). There should be no setup required beyond that.

### For test authors

There are two entry points for PSL tests. The tests involving writing plain
Rust tests, in `datamodel_tests.rs`, and the declarative validation tests, in
`validation.rs`.

For new tests, the validation test suite should be preferred: the tests are
more straightforward to write, more declarative (no unnecessary function calls,
test helpers, variable names, etc. that we always have to change at some
point), and much faster to compile.

A validation test is a `.prisma` file in the `tests/validation` directory, or
any of its subdirectories (transitively). It is a regular Prisma schema that
will be passed through the PSL validation process, and is expected to be valid,
or return errors. That expectation is defined by the comment (`//`) at the end
of the file. If there is no comment at the end of the file, the schema is
expected to be valid. If there is a comment at the end of the file, the
rendered, user-visible validation warnings and errors will be expected to match
the contents of that comment.

Like in `expect_test` tests, the last comment can be left out at first, and
updated like a snapshot test using the `UPDATE_EXPECT=1` env var.

