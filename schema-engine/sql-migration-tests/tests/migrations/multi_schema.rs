use std::borrow::Cow;

use schema_core::schema_connector::Namespaces;
use sql_migration_tests::test_api::*;
use test_setup::TestApiArgs;

/// Which schema to use during a `SchemaPush`. See `Schema` for more details.
pub enum WithSchema {
    First,
    Second,
}

/// Number of executed steps. See `CustomPushStep`.
pub enum ExecutedSteps {
    NonZero,
    Zero,
}

/// Represents the schemas we use during a `SchemaPush`.
///
/// Currently, the tests here have at most two schema push steps with different schemas, and at
/// least one.
///
/// In the case of `WithSchema::First`, the schema used would be the concatenation of `common` and
/// `first.
///
/// In the case of `WithSchema::Second`, the schema used would be the concatenation of `common` and
/// `second`.
pub struct Schema {
    pub common: String,
    pub first: String,
    pub second: Option<String>,
}

/// Used for `PushCustomAnd` for naming and disambiguation, represents a schema push step with
/// custom warnings, errors, etc.
pub struct CustomPushStep {
    pub warnings: &'static [&'static str],
    pub errors: &'static [&'static str],
    pub with_schema: WithSchema,
    pub executed_steps: ExecutedSteps,
}

/// This encapsulates setting up the database for the test, using a schema. It also potentially
/// runs multiple sequential updates.
///
/// This is essentially an ordered (linked) list of steps. Each step has a continuation except
/// `Done`, which is the list terminator.
///
/// There is currently a single way to interpret, via `run_schema_step`.
pub enum SchemaPush {
    /// Push the first or second schema and expect there are execution steps and no
    /// warnings/errors.
    PushAnd(WithSchema, &'static SchemaPush),
    /// Push with custom properties (warnings, errors, etc.).
    PushCustomAnd(CustomPushStep, &'static SchemaPush),
    /// Run a raw SQL command.
    RawCmdAnd(&'static str, &'static SchemaPush),
    /// Perform a (soft) reset.
    Reset(bool, &'static SchemaPush),
    /// List terminator.
    Done,
}

/// Represents a single test to be executed.
pub struct TestData {
    /// Name of the test.
    pub name: &'static str,
    /// Description of the test; should add some context and more details.
    pub description: &'static str,
    /// The schemas used in `SchemaPush`.
    pub schema: Schema,
    /// Namespaces that will be checked and must exist after running the push.
    pub namespaces: &'static [&'static str],
    /// Database setup through schema pushing, see `SchemaPush`.
    pub schema_push: SchemaPush,
    /// The assertion about tables, enums, etc.
    pub assertion: Box<dyn Fn(SchemaAssertion)>,
    /// Should we skip this test? None for yes, Some("reason") otherwise.
    pub skip: Option<&'static str>,
}

// Run a single test: create a new TestApi context, run the schema pushing, execute assertions.
pub fn run_test(test: &mut TestData) {
    let api_args = TestApiArgs::new("test", &[], &["one", "two"]);
    let mut api = TestApi::new(api_args);

    let mut vec_namespaces = test.namespaces.iter().map(|s| s.to_string()).collect();
    let namespaces = Namespaces::from_vec(&mut vec_namespaces);

    run_schema_step(&mut api, test, namespaces.clone(), &test.schema_push);

    let mut assertion = api.assert_schema_with_namespaces(namespaces);
    assertion.add_context(test.name);
    assertion.add_description(test.description);

    (test.assertion)(assertion)
}

// Recursively run schema steps.
pub fn run_schema_step(api: &mut TestApi, test: &TestData, namespaces: Option<Namespaces>, step: &SchemaPush) {
    match step {
        SchemaPush::PushAnd(first_or_second, next) => {
            let schema = match first_or_second {
                WithSchema::First => test.schema.common.to_owned() + test.schema.first.as_str(),
                WithSchema::Second => match &test.schema.second {
                    Some(base_second) => test.schema.common.to_owned() + base_second.as_str(),
                    None => panic!("Trying to run PushTwiceWithSteps but without defining the second migration."),
                },
            };
            api.schema_push(schema)
                .send()
                .with_context(String::from(test.name))
                .with_description(String::from(test.description))
                .assert_green()
                .assert_has_executed_steps();

            run_schema_step(api, test, namespaces, next);
        }

        SchemaPush::PushCustomAnd(
            CustomPushStep {
                warnings,
                errors,
                with_schema,
                executed_steps,
            },
            next,
        ) => {
            let schema = match with_schema {
                WithSchema::First => test.schema.common.to_owned() + test.schema.first.as_str(),
                WithSchema::Second => match &test.schema.second {
                    Some(base_second) => test.schema.common.to_owned() + base_second.as_str(),
                    None => panic!("Trying to run PushCustomAnd but without defining the second migration."),
                },
            };

            let warnings: Vec<Cow<str>> = warnings.iter().map(|s| (*s).into()).collect();
            let unexecutables: Vec<String> = errors.iter().map(|s| String::from(*s)).collect();

            let assert = api
                .schema_push(schema)
                .force(true)
                .send()
                .with_context(String::from(test.name))
                .with_description(String::from(test.description))
                .assert_warnings(warnings.as_slice())
                .assert_unexecutable(unexecutables.as_slice());

            match executed_steps {
                ExecutedSteps::NonZero => assert.assert_has_executed_steps(),
                ExecutedSteps::Zero => assert.assert_no_steps(),
            };

            run_schema_step(api, test, namespaces, next);
        }

        SchemaPush::RawCmdAnd(cmd, next) => {
            api.raw_cmd(cmd);
            run_schema_step(api, test, namespaces, next);
        }

        SchemaPush::Reset(soft, next) => {
            api.reset().soft(*soft).send_sync(namespaces.clone());
            run_schema_step(api, test, namespaces, next);
        }

        SchemaPush::Done => {}
    };
}
