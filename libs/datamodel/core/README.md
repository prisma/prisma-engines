# Prisma Schema Language implementation

## Overview

This crate is the public API for working with Prisma schemas: parsing and
validating their string representation, analyzing them, providing convenient
APIs, rendering, reformatting, etc. It is the main implementation of the Prisma
Schema Language, and it is used by all Prisma engines in this repository.

The query engine further processes Prisma schemas to produce the client API,
and the DMMF JSON format.

### Design goals

- Strict parsing: a duplicate attribute, unknown attribute, unknown argument or extra argument is an error.
- Expose a _convenient_ and _obvious_ / hard-to-misuse public API.
- Expose the source information (AST node spans, etc) in the parsed schema.
- Accumulate errors to present them at the end instead of returning early.

## Usage

Please see [`lib.rs`](src/lib.rs) and the [rustdoc documentation](https://prisma.github.io/prisma-engines/doc/datamodel/) for the public API.

Main use-case, parsing a string to datamodel:

```
let file = std::fs::read_to_string(&args[1]).unwrap();
let validated_schema = datamodel::parse_schema_parserdb(&file)?;
```

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
