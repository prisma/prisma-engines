# Query Engine Test Kit - A Full Guide
The test kit is focused on integration testing the query engine through request-response assertions.

## Test organization

The test kit is a combination of three crates, from which two are "lower level" crates that are only required to make it work, whereas only one is important if you only want to author tests.
```
               ┌────────────────────┐
           ┌───│ query-engine-tests │───┐
           │   └────────────────────┘   │
           ▼                            ▼
┌────────────────────┐       ┌────────────────────┐
│ query-test-macros  │       │ query-tests-setup  │
└────────────────────┘       └────────────────────┘
```

### `query-engine-tests`
The actual integration tests can be found in the `query-engine-tests` crate, specifically the `tests/` folder. The `src/` folder contains general utilities like time rendering, common schemas, string rendering, so everything that makes writing tests less painful.

Tests follow a `mod` tree like regular source files, with `query_engine_tests.rs` being the root. Ideally, the modules carry semantics on what is tested in the name and form coherent units that make it easy to spot and extend areas to test.

### `query-test-macros`
As the name implies, this crate contains the macro definitions used for the test suites (as shown later in this guide).

### `query-tests-setup`
Contains the main bulk of logic to make tests run, which is mostly invisible to the tests:
- Test configuration.
- Connector tags that know the connection strings, capabilities of connectors, how to render data models.
- Template parser of datamodels.
- Runners that know how to make requests against a certain query backend (+ the results that allow the tests to make assertions).
- Logging setup & error handling.

## Running tests
Tests are executed in the context of *one* _connector_ (with version) and _runner_. Some tests may only be specified to run for a subset of connectors or versions, in which case they will be skipped. Testing all connectors at once is not supported, however, for example, CI will run all the different connectors and versions concurrently in separate runs.

### Configuration

Tests must be configured to run There's a set of env vars that is always useful to have and an optional one.
Always useful to have:

```shell
export WORKSPACE_ROOT=/path/to/engines/repository/root
```

Test run env vars:
```shell
export TEST_CONNECTOR="postgres" # One of the supported providers.
export TEST_CONNECTOR_VERSION="10" # One of the supported versions.
```

As previously stated, the above can be omitted in favor of the `.test_config` config file:
```json
{
    "connector": "postgres",
    "version": "10"
}
```

The config file must be either in the current working folder from which you invoke a test run or in `$WORKSPACE_ROOT`.
It's recommended to use the file-based config as it's easier to switch between providers with an open IDE (reloading env vars would usually require reloading the IDE).
The workspace root makefile contains a series of convenience commands to setup different connector test configs, e.g. `make dev-postgres10` sets up the correct test config file for the tests to pick up.

On the note of docker containers: Most connectors require an endpoint to run against (notable exception at the moment is SQLite), so you need to provide one. The `docker-compose.yml` in the workspace root offers all possible databases and versions we actively test. The aforementioned `make` commands also set up the container for you together with the .

If you choose to set up the databases yourself, please note that the connection strings used in the tests (found in the files in `<repo_root>/query-engine/connector-test-kit-rs/query-tests-setup/src/connector_tag/`) to set up user, password and database for the test user.

### Running

Note that by default tests run concurrently.

- VSCode should automatically detect tests and display `run test`.
- Use `make test-qe` (minimal log output) or `make test-qe-verbose` (all log output) in `$WORKSPACE_ROOT`.
- `cargo test` in the `query-engine-tests` crate.
- A single test can be tested with the normal cargo rust facilities from command line, e.g. `cargo test --package query-engine-tests --test query_engine_tests --all-features -- queries::filters::where_unique::where_unique::no_unique_fields --exact --nocapture` where `queries::filters::where_unique::where_unique::no_unique_fields` can be substituted for the path you want to test.
- If you want to test a single relation test, define the `RELATION_TEST_IDX` env var with its index.

#### Running tests through driver adapters

The query engine is able to delegate query execution to javascript through driver adapters.
This means that instead of drivers being implemented in Rust, it's a layer of adapters over NodeJs
drivers the code that actually communicates with the databases. See [`adapter-*` packages in prisma/prisma](https://github.com/prisma/prisma/tree/main/packages)

To run tests through a driver adapters, you should also configure the following environment variables:

* `DRIVER_ADAPTER`: tells the test executor to use a particular driver adapter. Set to `neon`, `planetscale` or any other supported adapter.
* `DRIVER_ADAPTER_CONFIG`: a json string with the configuration for the driver adapter. This is adapter specific. See the [github workflow for driver adapter tests](.github/workflows/query-engine-driver-adapters.yml) for examples on how to configure the driver adapters.
* `ENGINE`: can be used to run either `wasm` or `napi` or `c-abi` version of the engine.

Example:

```shell
export EXTERNAL_TEST_EXECUTOR="$WORKSPACE_ROOT/query-engine/driver-adapters/executor/script/testd.sh"
export DRIVER_ADAPTER=neon
export ENGINE=wasm
export DRIVER_ADAPTER_CONFIG ='{ "proxyUrl": "127.0.0.1:5488/v1" }'
````

We have provided helpers to run the query-engine tests with driver adapters, these helpers set all the required environment
variables for you:

```shell
DRIVER_ADAPTER=$adapter ENGINE=$engine make test-qe
```

Where `$adapter` is one of the supported adapters: `neon`, `planetscale`, `libsql`.


## Authoring tests
The following is an example on how to write a new test suite, as extending or changing an existing one follows the same rules and considerations.

### Find a suitable place for the module
For example if you choose `tests/queries/filters/some_spec.rs`, you create the file and add the module to the `filters/mod.rs`.
The modules usually follow a tree structure that convey some sort of meaning on what is tested. If you're unsure ping Dom.

### Decide on the test layout
_Option 1:_
```rust
use query_engine_tests::*;

#[test_suite(...)]
mod some_spec {
    #[connector_test(...)]
    async fn my_test(runner: Runner) -> TestResult<()> {
        // ...
        Ok(())
    }
}
```

_Option 2:_
```rust
use query_engine_tests::*;

#[connector_test(...)]
async fn my_test(runner: Runner) -> TestResult<()> {
    // ...
    Ok(())
}
```

Note that regardless of the shape, `connector_test` tests must _always_ have the signature of `async fn test_name(runner: Runner) -> TestResult<()>`.

Option 1 uses a `mod` that can be used to define common attributes on all tests contained in the module. Option 2 doesn't use a `mod` and requires you to set more attributes per test (will be shown later). Option 1 produces a test like `queries::filters::some_spec::some_spec::my_test` and option 2 `queries::filters::some_spec::my_test` (note the double `some_spec`). Apart from the aesthetics in the naming, there are no other consequences for using option 1 over 2. You can also choose any `mod` name you wish, e.g. `mod my_specs` would produce `queries::filters::some_spec::my_specs::my_test`.

Why is this important? The macro attributes used above define all of the properties of the tests that are used to determine how and when they are run.
The full attribute definitions are as follows:
```rust
#[test_suite(
    schema(schema_handler),
    exclude(Connector(Version1, ...), Connector, ...),
    only(Connector(Version1, ...), Connector, ...),
    capabilities(Capability1, Capability2, ...)
)]
```

```rust
#[connector_test(
    suite = "name", // Required (Optional if in a `test_suite` mod)
    schema(schema_handler), // Required either on the mod or on the test itself.
    exclude(Connector(Version1, ...), Connector, ...),
    only(Connector(Version1, ...), Connector, ...),
    capabilities(Capability1, Capability2, ...)
)]
```

The definitions can have almost the same properties, and the used properties are identical in meaning, however `connector_test` depends on `test_suite` if both are present. The basic rule is: Everything that is set on `test_suite` is propagated to every `connector_test` in the module, except if it's already set on `connector_test`.

For example, if `schema` is set on `test_suite`, then all contained connector tests that do not have a `schema` property of their own implicitly have the `schema` attribute set.
There are two special cases in that rule:
- `suite`: No explicit `suite = "..."` is required on `test_suite`, as every `connector_test` in a `test_suite` has the name of the `mod` (eg. "some_spec" for `mod some_spec`) set (again, only if not `suite = "..."` is set already).
- `only` and `exclude` are mutually exclusive (more details below), which means that if one of the two is set on `connector_test` already, the other will not implicitly propagate to `connector_test` from `test_suite`:
```rust
#[test_suite(only(Postgres))]
mod some_spec {
    // Will run for everything except SQL Server 2017. The `only` of `test_suite` is not propagated!
    #[connector_test(exclude(SqlServer(2017))]
    async fn my_test(runner: Runner) -> TestResult<()> {
        // ...
        Ok(())
    }
}
```

Let's take a look at what the properties mean:

**`schema`**
The _schema handler_ to use for the test, given as a path ending in a function pointer. This is *always required* to be present on a test. A schema handler produces the schema that the test tests against. The path must be resolvable from the scope of the test function.
```rust
#[test_suite(schema(schema_handler))]
mod some_spec {
    fn schema_handler() -> String {
        "model A {
            #id(id, Int, @id)
            field String?
        }"
        .to_owned()
    }

    #[connector_test]
    async fn my_test(runner: Runner) -> TestResult<()> {
        // Assertions against the models as given by the handler.
        Ok(())
    }
}
```
Note that the schema handlers can be located anywhere, the only important bit is that they're in scope for the test:
`#[test_suite(schema(some_other_mod::path::schema_handler))]`

**`exclude` and `only`**
Mutually exclusive properties that constrain tests to run only for a set of connectors. By default (when none of the two are given), _all_ possible connectors are run for a test. `only` sets a whitelist of connectors to run, `exclude` sets a blacklist. The values used in both are identical in form: `Connector` or `Connector(Version, Version, ...)`. If no version is given, the entire connector family (all versions) is included or excluded.

Connectors are at the time of writing:
- `Postgres`: 9.6, 10, 11, 12, 13, 14, 15
- `MySql`: 5.6, 5.7, 8, mariadb
- `SqlServer`: 2017, 2019, 2022
- `Sqlite`: No versions
- `MongoDb`: 4

Connector tags can be written all lowercase, uppercase, camel, doesn't matter. Versions can be written as literal, string, float, int. A few examples:

`only(Postgres, MySql(5.7, "5.6"))`: All Postgres versions + MySql 5.6 and 5.7
`exclude(SqlServer)`: All connectors except all versions of SqlServer.
`only(SQLSERVER)`: All versions of SQL Server, nothing else.

**`capabilities`**
Requires connectors to have _all_ of the given connector capabilities (for a full list of valid capabilities see `pub enum ConnectorCapability` in `datamodel-connector/src/lib.rs`). Note that you can give both connectors and capabilities, but if the connectors you specify do not have the capabilities, the test(s) will be skipped.

Example: `capabilities(ScalarLists, CreateMany)`.

#### A Word on Test Execution
Tests are running concurrently by default, which makes it necessary to isolate them from each other.
For that purpose, each test runs against a separate sandbox in the underlying connector. In MySQL, this is a separate database, in Sqlite this is a separate database file.
The name of the "sandbox" is defined as `suite` + '_' + `test function name`.

A minimal test example is:
```rust
#[test_suite(schema(some_handler))]
mod some_spec {
    #[connector_test]
    async fn my_test(runner: Runner) -> TestResult<()> {
        // Assertions against the models as given by the handler.
        Ok(())
    }
}
```

The test database in MySQL would be `some_spec_my_test`, the file for Sqlite `some_spec_my_test.db`.
For details on how each connector handles it, look into the files in `query-tests-setup/src/connector_tag` where the connection strings are rendered.

### Writing Schema templates & Common Schemas
Schemas that are used for tests that are supposed to run for all connectors must be templated. Currently, MongoDb requires parts of a schema to have different forms, which would require writing all schemas twice, or duplicating tests for Mongo, etc.

For this reason, schemas have template strings of the form `#name(args)` embedded in them. Connectors decide how to render schemas (see `query-tests-setup/src/datamodel_rendering` for details). Currently two templates are available:
- `#id(field_name, field_type, directives ...)` - For defining an ID field on a model.
    - `#id(pid, Int, @id, @map("_pid"))`
- `#m2m(field_name, field_type, opposing_field_name, opposing_type, opt_relation_name)` - For defining a many-to-many relation between two models.
    - Example: `#m2m(posts, Post[], id, String, "name")`

All SQL connectors render these with a standard `SqlDatamodelRenderer`, Mongo uses its own `MongoDbSchemaRenderer`.

Consider using one of the common schemas located in `query-engine-tests/sec/schemas` to write your tests - they are already templated correctly and reducing the number of schemas used overall helps keeping the tests more compact. However, if a test suite requires a specialized schema, it's totally fine to write one yourself, just remember to template it correctly.

## Assertions

Some utils are available to ease assertions. You'll find below how to use each of them.

### `insta::assert_snapshot!`

In most tests, we simply run a query and expect an output. Snapshots are extremely convenient to automatically generate the expected output of an assertion, directly in your code. Here's how the flow goes:

#### Step 1 - (Optional) Install `cargo-insta`

If you want to easily review/update/generate your snapshots, it is recommended to install `cargo-insta` via `cargo install cargo-insta`.

`cargo-insta` is also useful to run all the tests without having them stop at the first failure. Instead, it will collect all the failing snapshots and let you review them in a batch after your tests are done running.

If you prefer, you can follow [this short video](https://www.youtube.com/watch?v=rCHrMqE4JOY) which explains how to use the tool.

#### Step 2 - Create the test

We intentionally leave the expected output empty as you can see below.

```rs
#[connector_test]
async fn some_test(runner: Runner) -> TestResult<()> {
    insta::assert_snapshot!(
        run_query!(&runner, r#"<your_query>"#),
        @""
    );

    Ok(())
}
```

#### Step 3 - Run your tests

##### With `cargo-insta`

  - If you want to run all the tests without them stoping at the first failure, use  `cargo insta test --package query-engine-tests`. (Caveat: all the tests have to be run). There's a couple of handy additional flags that can be passed to the command (such as `--review`). Use `cargo insta test --help` to know more about them.

  - If you want to run a single test or test-suite, use `cargo test` as usual. Pay attention to the end of the output log. You should see something like `info: X snapshot to review`. See Step 4 for the review process.

##### Without `cargo-insta`

If you haven't installed `cargo-insta`, use `cargo test` as usual.

#### Step 4 - Review the snapshots

##### With `cargo-insta`

> ⚠️ **Important**: While automatic snapshot updates are extremely convenient, it is also an easy way to miss unintended changes. **Please, don't ever just update all your snapshots to make the CI green without carefully checking what was changed and whether that was the intended change.**

Run `cargo insta review` to be prompted with an interactive view that lets you accept or reject the snapshots changes if there are any. Below is an example of a failing snapshot:

```
Package: query-engine-tests (0.1.0)
Snapshot: avg_with_all_sorts_of_query_args-6
Source: query-engine/connector-test-kit-rs/query-engine-tests/tests/queries/aggregation/avg.rs:67
-old snapshot
+new results
──────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────
run_query!(
    runner,
    r#"query { aggregateTestModel(cursor: { id: 3 }) { avg { int bInt float decimal } } }"#
)
────────────┬─────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────
    0       │-{"data":{"aggregateTestModel":{"_avg":{"int":1.5,"bInt":1.5,"float":0.75,"decimal":"0.75"}}}}
          0 │+{"data":{"aggregateTestModel":{"avg":{"int":1.5,"bInt":1.5,"float":0.75,"decimal":"0.75"}}}}
────────────┴─────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────

  a accept   keep the new snapshot
  r reject   keep the old snapshot
  s skip     keep both for now
```

- Press `a` to accept the snapshot update
- Press `r` to reject the snapshot update
- Press `s` to skip the snapshot update

**Accepting a snapshot update will replace, directly in your code, the expected output in the assertion.**

If you dislike the interactive view, you can also run `cargo insta accept` to automatically accept all snapshots and then use your git diff to check if everything is as intented.

##### Without `cargo-insta`

If you haven't installed `cargo-insta`, have a look at the error output and manually update the snapshot if the change is expected, just like when using `assert_eq!`.


### Adding a new data store source for tests

Let's say you already have connector tests for MongoDB but right now it runs only with version 4.4 and want to support version 5.0, the steps are easy but requires changes in different places to be sure we run the tests everywhere.

1. Add the container image for your new data store source to the `docker-compose.yml` file, name it to something you will remember, for example `mongo5`
2. Create a connector file in the `query-engine/connector-test-kit-rs/test-configs/` with the connector data (see other examples in that director), name it with something that makes sense, for example `mongo5`
3. Add the credentials to access the _data store service_ from the docker compose file, this is done creating the required file in `.test_database_urls`, for example `.test_database_urls/mongo5`
4. Make sure this image is available to build and prepare the environment in the `Makefile`, in the query engine we depend in two Make targets, `dev-` and `start-`
   - The `start-` target (for example `start-mongo5`) will execute the _data store service_ in docker compose, for example `docker compose -f docker-compose.yml up -d --remove-orphans mongo5`
   - The `dev-` target (for example `dev-mongo5`) will depend on the `start-` target and copy the correct _connector file_, for example `cp $(CONFIG_PATH)/mongodb5 $(CONFIG_FILE)`
5. Add the new test data store source to the `query-engine/connector-test-kit-rs/query-test-setup/src/connector_tag` file, if it is a completely new data store create the required file, in our case we need to modify `mongodb.rs`
   - Add the new version to the version enum (ex. `MongoDbVersion`)
   - Implement or amend the `try_from` function for the version enum
   - Implement or amend the `to_string` function for the version enum
   - Add the new source to the `connection_string` method. You need two implementations, one for the internal CI and another for the local test
6. Add the new data source as a given capability, without this your test specific to that version won't run at all. For this you need to add the capability to the vector in the `all` function.
