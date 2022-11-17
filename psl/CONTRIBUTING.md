# Contributing to datamodel

## Testing

### Running the test suite

Run `cargo test` in the `psl` crate. There should be no setup required beyond that.

### For test authors

There are three entry points for PSL tests:

- the tests involving writing plain Rust tests, in [psl/tests/datamodel_tests.rs]
- the declarative validation tests, in [psl/tests/validation_tests.rs]
- the declarative reformatting tests, in [psl/tests/reformat_tests.rs].

#### Validation tests

For new tests where we only want to check that a given Prisma schema produces
validation errors (or none at all), the validation test suite should be
preferred: the tests are more straightforward to write, more declarative (no
unnecessary function calls, test helpers, variable names, etc. that we always
have to change at some point), and much faster to compile.

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

#### Reformatter tests

These work similarly to the validation tests, with the same snapshot mechanism with the `UPDATE_EXPECT` env var.

For each Prisma schema in the `reformatter`, directory, the test harness will assert that its contents match the file with the same name, but with the `reformatted.prisma` extension.

The test harness also reformats _that_ file's contents in turn to check that the reformatting is idempotent.

## Style guidelines

- Avoid unnecessary object-like structs. Use free-standing functions and context structs.
- Function arguments should generally be ordered from more specific to less
  specific. Any context-like arguments should come last. Mutable arguments also
  should tend to come last, since they're for generally for writing
  (side-effects, context) rather than reading.
