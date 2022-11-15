use std::borrow::Cow;

use indoc::indoc;
use migration_core::migration_connector::Namespaces;
use migration_engine_tests::test_api::*;
use test_setup::TestApiArgs;

struct Schema {
    common: String,
    first: String,
    second: Option<String>,
}

enum SchemaPush {
    PushAnd(bool, &'static SchemaPush),
    PushCustomAnd(&'static [&'static str], &'static [&'static str], bool, bool, &'static SchemaPush),
    RawCmdAnd(&'static str, &'static SchemaPush),
    Reset(bool, &'static SchemaPush),
    Done,
}

struct TestData {
    name: &'static str,
    description: &'static str,
    schema: Schema,
    namespaces: &'static [&'static str],
    schema_push: SchemaPush,
    assertion: Box<dyn Fn(SchemaAssertion) -> ()>,
    skip: Option<String>,
}

#[test_connector(
    tags(Postgres),
    exclude(CockroachDb),
    preview_features("multiSchema"),
    namespaces("one", "two")
)]
fn multi_schema_tests(_api: TestApi) {
    let namespaces : &'static [&'static str] = &["one", "two"];
    let base_schema = indoc! {r#"
        datasource db {
          provider   = "postgresql"
          url        = env("TEST_DATABASE_URL")
          schemas    = ["one", "two"]
        }

        generator js {
          provider        = "prisma-client-js"
          previewFeatures = ["multiSchema"]
        } "#};

    let mut tests = [
        TestData {
            name: "basic",
            description: "Test single migration on two custom namespaces with a table each.",
            schema: Schema {
                common: base_schema.into(),
                first: indoc! {r#"
        model First {
          id Int @id
          @@schema("one")
        }

        model Second {
          id Int @id
          @@schema("two")
        } "#}.into(),
                second: None,
            },
            namespaces: &namespaces,
            schema_push: SchemaPush::PushAnd(true, &SchemaPush::Done),
            assertion: Box::new(|assert| {
                assert.assert_has_table("First").assert_has_table("Second");
            }),
            skip: None,
        },
        TestData {
            name: "idempotence",
            description: "Test idempotence test with two namespaces and a table each",
            schema: Schema {
                common: base_schema.into(),
                first: indoc! {r#"
        model First {
          id Int @id
          @@schema("one")
        }

        model Second {
          id Int @id
          @@schema("two")
        }"#}.into(),
                second: None,
            },
            namespaces: &namespaces,
            schema_push: SchemaPush::PushAnd(true,
                           &SchemaPush::PushCustomAnd(&[], &[], true, false,
                             &SchemaPush::Done)),
            assertion: Box::new(|assert| {
                assert.assert_has_table("First").assert_has_table("Second");
            }),
            skip: None,
        },
        TestData {
            name: "add table",
            description: "Test adding a new table to one of the namespaces",
            schema: Schema {
                common: (base_schema.to_owned() + indoc! {r#"
        model First {
          id Int @id
          @@schema("one")
        }

        model Second {
          id Int @id
          @@schema("two")
        }"#}),
                first: "".into(),
                second: Some(
                    indoc! {r#"
        model Third {
          id Int @id
          @@schema("one")
        } "#}.into(),
                ),
            },
            namespaces: &namespaces,
            schema_push: SchemaPush::PushAnd(true, &SchemaPush::PushAnd(false, &SchemaPush::Done)),
            assertion: Box::new(|assert| {
                assert
                    .assert_has_table("First")
                    .assert_has_table("Second")
                    .assert_has_table("Third");
            }),
            skip: None,
        },
        TestData {
            name: "remove table",
            description: "Test removing a table to one of the namespaces",
            schema: Schema {
                common: (base_schema.to_owned() + indoc! {r#"
        model First {
          id Int @id
          @@schema("one")
        }"#}),
                first: indoc! {r#"
        model Second {
          id Int @id
          @@schema("two")
        } "#}.into(),
                second: Some(" ".into()),
            },
            namespaces: &namespaces,
            schema_push: SchemaPush::PushAnd(true, &SchemaPush::PushAnd(false, &SchemaPush::Done)),
            assertion: Box::new(|assert| {
                assert
                    .assert_has_table("First")
                    .assert_has_no_table("Second");
            }),
            skip: None,
        },
        TestData {
            name: "recreate not null column with non-null values",
            description: "Test dropping a nullable column and recreating it as non-nullable, given a row exists with a non-NULL value",
            schema: Schema {
                common: (base_schema.to_owned() + indoc! {r#"
        model First {
          id Int @id
          @@schema("one")
        }"#}),
                first: indoc! {r#"
        model Second {
          id Int @id
          name String?
          @@schema("two")
        }"#}.into(),
                second: Some(indoc!{r#"
        model Second {
          id Int @id
          name String
          @@schema("two")
        }"#}.into()),
            },
            namespaces: &namespaces,
            schema_push: SchemaPush::PushAnd(true,
                           &SchemaPush::RawCmdAnd("INSERT INTO \"two\".\"Second\" VALUES(1, 'some value');",
                             &SchemaPush::PushAnd(false, &SchemaPush::Done))),
            assertion: Box::new(|assert| {
                assert
                    .assert_has_table("First")
                    .assert_has_table("Second");
            }),
            skip: None,
        },
        TestData {
            name: "recreate not null column with null values",
            description: "Test dropping a nullable column and recreating it as non-nullable, given a row exists with a NULL value",
            schema: Schema {
                common: (base_schema.to_owned() + indoc! {r#"
        model First {
          id Int @id
          @@schema("one")
        }"#}),
                first: indoc! {r#"
        model Second {
          id Int @id
          name String?
          @@schema("two")
        }"#}.into(),
                second: Some(indoc!{r#"
        model Second {
          id Int @id
          name String
          @@schema("two")
        }"#}.into()),
            },
            namespaces: &namespaces,
            schema_push: SchemaPush::PushAnd(true,
                           &SchemaPush::RawCmdAnd("INSERT INTO \"two\".\"Second\" VALUES(1, NULL);",
                             &SchemaPush::PushCustomAnd(&[], &["Made the column `name` on table `Second` required, but there are 1 existing NULL values."], false, false,
                               &SchemaPush::Done))),
            assertion: Box::new(|assert| {
                assert
                    .assert_has_table("First")
                    .assert_has_table("Second");
            }),
            skip: None,
        },
        TestData {
            name: "add required field",
            description: "Test adding a required field to a table with no records",
            schema: Schema {
                common: (base_schema.to_owned() + indoc! {r#"
        model First {
          id Int @id
          @@schema("one")
        }"#}),
                first: indoc! {r#"
        model Second {
          id Int @id
          @@schema("two")
        }"#}.into(),
                second: Some( indoc! {r#"
        model Second {
          id Int @id
          name String
          @@schema("two")
        }"#}.into()),
            },
            namespaces: &namespaces,
            schema_push: SchemaPush::PushAnd(true, &SchemaPush::PushAnd(false, &SchemaPush::Done)),
            assertion: Box::new(|assert| {
                assert
                    .assert_has_table("First")
                    .assert_has_table("Second");
            }),
            skip: None,
        },
        TestData {
            name: "change field type to array",
            description: "Test changing a field type to array.",
            schema: Schema {
                common: (base_schema.to_owned() + indoc! {r#"
        model First {
          id Int @id
          @@schema("one")
        }"#}),
                first: indoc! {r#"
        model Second {
          id Int @id
          name String
          @@schema("two")
        }"#}.into(),
                second: Some( indoc! {r#"
        model Second {
          id Int @id
          name String[]
          @@schema("two")
        }"#}.into()),
            },
            namespaces: &namespaces,
            schema_push: SchemaPush::PushAnd(true, &SchemaPush::PushAnd(false, &SchemaPush::Done)),
            assertion: Box::new(|assert| {
                assert
                    .assert_has_table("First")
                    .assert_has_table("Second");
            }),
            skip: None,
        },
        TestData {
            name: "change field type from array",
            description: "Test changing a field type from array.",
            schema: Schema {
                common: (base_schema.to_owned() + indoc! {r#"
        model First {
          id Int @id
          @@schema("one")
        }"#}),
                first: indoc! {r#"
        model Second {
          id Int @id
          name String[]
          @@schema("two")
        }"#}.into(),
                second: Some( indoc! {r#"
        model Second {
          id Int @id
          name String
          @@schema("two")
        }"#}.into()),
            },
            namespaces: &namespaces,
            schema_push: SchemaPush::PushAnd(true, &SchemaPush::PushAnd(false, &SchemaPush::Done)),
            assertion: Box::new(|assert| {
                assert
                    .assert_has_table("First")
                    .assert_has_table("Second");
            }),
            skip: None,
        },
        TestData {
            name: "rename index",
            description: "Test renaming an index.",
            schema: Schema {
                common: (base_schema.to_owned() + indoc! {r#"
        model First {
          id Int @id
          @@schema("one")
        }"#}),
                first: indoc! {r#"
        model Second {
          id Int @id
          name String
          @@index(fields: [name], map: "index_name")
          @@schema("two")
        }"#}.into(),
                second: Some( indoc! {r#"
        model Second {
          id Int @id
          name String
          @@index(fields: [name], map: "new_index_name")
          @@schema("two")
        }"#}.into()),
            },
            namespaces: &namespaces,
            schema_push: SchemaPush::PushAnd(true, &SchemaPush::PushAnd(false, &SchemaPush::Done)),
            assertion: Box::new(|assert| {
                assert
                    .assert_has_table("First")
                    .assert_has_table("Second");
            }),
            skip: None,
        },
        TestData {
            name: "add unique to column",
            description: "Test adding the unique flag to a column.",
            schema: Schema {
                common: (base_schema.to_owned() + indoc! {r#"
        model First {
          id Int @id
          @@schema("one")
        }"#}),
                first: indoc! {r#"
        model Second {
          id Int @id
          name String
          @@schema("two")
        } "#}.into(),
                second: Some( indoc! {r#"
        model Second {
          id Int @id
          name String @unique
          @@schema("two")
        }"#}.into()),
            },
            namespaces: &namespaces,
            schema_push: SchemaPush::PushAnd(true,
                           &SchemaPush::PushCustomAnd(&["A unique constraint covering the columns `[name]` on the table `Second` will be added. If there are existing duplicate values, this will fail."], &[], false, false,
                             &SchemaPush::Done)),
            assertion: Box::new(|assert| {
                assert
                    .assert_has_table("First")
                    .assert_has_table("Second");
            }),
            skip: None,
        },
        TestData {
            name: "drop enum",
            description: "Test removing an enum from a namespace.",
            schema: Schema {
                common: (base_schema.to_owned() + indoc! {r#"
        model First {
          id Int @id
          @@schema("one")
        }
    "#}),
                first: indoc! {r#"
        enum Second {
          One
          Two
          @@schema("two")
        } "#}.into(),
                second: Some( indoc! {r#""#}.into()),
            },
            namespaces: &namespaces,
            schema_push: SchemaPush::PushAnd(true, &SchemaPush::PushAnd(false, &SchemaPush::Done)),
            assertion: Box::new(|assert| {
                assert
                    .assert_has_table("First")
                    .assert_has_no_enum("Second");
            }),
            skip: None,
        },
        TestData {
            name: "drop foreign key",
            description: "Test removing a foreign key from a namespace.",
            schema: Schema {
                common: base_schema.to_owned(),
                first: indoc! {r#"
        model First {
          id Int @id
          seconds Second[]
          @@schema("one")
        }
        model Second {
          id Int @id
          first_id Int
          first First? @relation(fields: [first_id], references: [id])
          @@schema("one")
        }"#}.into(),
                second: Some( indoc! {r#"
        model First {
          id Int @id
          @@schema("one")
        }
        model Second {
          id Int @id
          @@schema("one")
        } "#}.into()),
            },
            namespaces: &namespaces,
            schema_push: SchemaPush::PushAnd(true, &SchemaPush::PushAnd(false, &SchemaPush::Done)),
            assertion: Box::new(|assert| {
                assert
                    .assert_has_table("First")
                    .assert_has_table("Second");
            }),
            skip: None,
        },
        TestData {
            name: "drop index",
            description: "Test removing an index from a namespace.",
            schema: Schema {
                common: (base_schema.to_owned() + indoc! {r#"
        model First {
          id Int @id
          @@schema("one")
        }"#}),
                first: indoc! {r#"
        model Second {
          id Int @id
          name String
          @@index(fields: [name], map: "index_name")
          @@schema("two")
        }"#}.into(),
                second: Some( indoc! {r#"
        model Second {
          id Int @id
          name String
          @@schema("two")
        } "#}.into()),
            },
            namespaces: &namespaces,
            schema_push: SchemaPush::PushAnd(true, &SchemaPush::PushAnd(false, &SchemaPush::Done)),
            assertion: Box::new(|assert| {
                assert
                    .assert_has_table("First")
                    .assert_has_table("Second");
            }),
            skip: None,
        },
        TestData {
            name: "drop view",
            description: "Test removing a view via reset from a namespace.",
            schema: Schema {
                common: (base_schema.to_owned() + indoc! {r#"
        model First {
          id Int @id
          name String
          @@schema("one")
        }"#}),
                first: indoc! {r#""#}.into(),
                second: None,
            },
            namespaces: &namespaces,
            schema_push: SchemaPush::PushAnd(true,
                           &SchemaPush::RawCmdAnd("CREATE VIEW \"two\".\"Test\" (id, name) as SELECT id, name FROM \"one\".\"First\"",
                             &SchemaPush::Reset(true,
                               &SchemaPush::Done))),
            assertion: Box::new(|assert| {
                assert.assert_views_count(0);
            }),
            skip: None,
        },
        TestData {
            name: "alter view",
            description: "Test adding a variant to an enum in a namespace.",
            schema: Schema {
                common: base_schema.to_string(),
                first: indoc! {r#"
      enum SomeEnum {
        First
        Second
        @@schema("one")
      }"#}.into(),
                second: Some(indoc! {r#"
      enum SomeEnum {
        First
        Second
        Third
        @@schema("one")
      }"#}.into()),
            },
            namespaces: &namespaces,
            schema_push: SchemaPush::PushAnd(true, &SchemaPush::PushAnd(false, &SchemaPush::Done)),
            assertion: Box::new(|assert| {
                assert.assert_enum("SomeEnum", |e| e.assert_values(&["First", "Second", "Third"]));
            }),
            skip: None,
        },
        TestData {
            name: "move enum across namespaces",
            description: "Test moving an enum to a different namespace.",
            schema: Schema {
                common: base_schema.to_string(),
                first: indoc! {r#"
      enum SomeEnum {
        First
        Second
        @@schema("one")
      }"#}.into(),
                second: Some(indoc! {r#"
      enum SomeEnum {
        First
        Second
        @@schema("two")
      }"#}.into()),
            },
            namespaces: &namespaces,
            schema_push: SchemaPush::PushAnd(true, &SchemaPush::PushAnd(false, &SchemaPush::Done)),
            assertion: Box::new(|assert| {
                assert.assert_enum("SomeEnum", |e| e.assert_values(&["First", "Second", "Third"]));
            }),
            skip: Some("TODO".into()),
        },
    ];

    // traverse_ is always the answer
    tests.iter_mut().filter(|t| t.skip.is_none()).for_each(|mut t| {
        run_test(&mut t);
    });
}

fn run_test(test: &mut TestData) {
    let api_args = TestApiArgs::new("test", &["multiSchema"], &["one", "two"]);
    let mut api = TestApi::new(api_args);

    let mut vec_namespaces = test.namespaces.iter().map(|s| s.to_string()).collect();
    let namespaces = Namespaces::from_vec(&mut vec_namespaces);

    run_schema_step(&mut api, test, namespaces.clone(), &test.schema_push);

    let mut assertion = api.assert_schema_with_namespaces(namespaces);
    assertion.add_context(test.name.to_string());
    assertion.add_description(test.description.to_string());

    (test.assertion)(assertion)
}

fn run_schema_step(api: &mut TestApi, test: &TestData, namespaces: Option<Namespaces>, step: &SchemaPush) {
    let first = test.schema.common.to_owned() + test.schema.first.as_str();
    match step {
        SchemaPush::PushAnd(is_first, next) => {
            let schema = if *is_first {
                first
            } else {
                match &test.schema.second {
                    Some(base_second) => test.schema.common.to_owned() + base_second.as_str(),
                    None => panic!("Trying to run PushTwiceWithSteps but without defining the second migration."),
                }
            };
            api.schema_push(schema)
                .send()
                .with_context(String::from(test.name))
                .with_description(String::from(test.description))
                .assert_green()
                .assert_has_executed_steps();
            run_schema_step(api, test, namespaces, next);
        }
        SchemaPush::PushCustomAnd(warnings, unexecutable, is_first, has_steps, next) => {
            let schema = if *is_first {
                first
            } else {
                match &test.schema.second {
                    Some(base_second) => test.schema.common.to_owned() + base_second.as_str(),
                    None => panic!("Trying to run PushTwiceWithSteps but without defining the second migration."),
                }
            };
            let warnings: Vec<Cow<str>> = warnings.iter().map(|s| (*s).into()).collect();
            let unexecutables: Vec<String> = unexecutable.iter().map(|s| String::from(*s)).collect();
            let assert = api
                .schema_push(schema)
                .send()
                .with_context(String::from(test.name))
                .with_description(String::from(test.description))
                .assert_warnings(warnings.as_slice())
                .assert_unexecutable(unexecutables.as_slice());
            if *has_steps {
                assert.assert_has_executed_steps();
            } else {
                assert.assert_no_steps();
            }
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

