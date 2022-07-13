# Introspection engine tests

There are two entry points for introspection tests. The tests involving writing
plain Rust tests, in `introspection_tests.rs`, and the simple test setup, in
`simple.rs`.

## The simple test setup

Many test cases only involve setting up a database schema with SQL, then
introspecting that database. We want to keep this kind of tests as declarative
as possible.

The so-called simple test suite consists of single SQL files, that will be run
as-is. In addition to the SQL content, comments are used for configuration and
expectations.

All the SQL files under the `tests/simple` directory and all its subdirectories
will be run, in parallel and in no particular order.

The SQL test file **must** start with a comment specifying which tags the test
applies to. It **may** be followed by a line specifying which tags to exclude.
The tags are the same as for the regular test setup.

Example:

```sql
-- tags=postgres
-- exclude=cockroachdb
```

The file **must** also end with a block SQL comment (`/*\n ... \n*/\n`). This
block comment contains the expected introspected schema.

Like `expect_test` tests, the last block comment can be left empty at first,
and updated like a snapshot test using the `UPDATE_EXPECT=1` env var.
