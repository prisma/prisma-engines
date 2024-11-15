use indoc::{formatdoc, indoc};
use psl::PreviewFeature;
use schema_core::{
    commands::{apply_migrations, create_migration},
    json_rpc::types::{ApplyMigrationsInput, CreateMigrationInput, SchemasContainer},
    schema_connector::{ConnectorParams, SchemaConnector},
};
use sql_schema_connector::SqlSchemaConnector;
use url::Url;

use crate::migrations::multi_schema::*;
use sql_migration_tests::test_api::*;
use sql_schema_describer::DefaultValue;

#[test_connector(tags(Postgres), preview_features("multiSchema"), namespaces("one", "prisma-tests"))]
fn apply_migrations_with_multiple_schemas_where_one_is_search_path_with_a_foreign_key(api: TestApi) {
    let datasource = api.datasource_block_with(&[("schemas", r#"["one", "prisma-tests"]"#)]);
    let generator = api.generator_block();

    let dm = indoc::formatdoc! {r#"
        {datasource}

        {generator}

        model A {{
           id Int @id
           bs B[]

           @@schema("one")
        }}

        model B {{
           id  Int  @id
           aId Int
           a   A    @relation(fields: [aId], references: [id])

           @@schema("prisma-tests")
        }}
    "#};

    let dir = api.create_migrations_directory();

    api.create_migration("init", &dm, &dir).send_sync();

    api.apply_migrations(&dir)
        .send_sync()
        .assert_applied_migrations(&["init"]);

    api.apply_migrations(&dir).send_sync().assert_applied_migrations(&[]);
}

// This is the only "top" level test in this module. It defines a list of tests and executes them.
// If you want to look at the tests, see the `tests` variable below.
#[test_connector(tags(Postgres), preview_features("multiSchema"), namespaces("one", "two"))]
fn multi_schema_tests(_api: TestApi) {
    let namespaces: &'static [&'static str] = &["one", "two"];
    let base_schema = indoc! {r#"
        datasource db {
          provider   = "postgresql"
          url        = env("TEST_DATABASE_URL")
          schemas    = ["one", "two"]
        }

        generator js {
          provider        = "prisma-client-js"
          previewFeatures = ["multiSchema"]
        }
    "#};

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
            namespaces,
            schema_push: SchemaPush::PushAnd(WithSchema::First, &SchemaPush::Done),
            assertion: Box::new(|assert| {
                assert.assert_has_table_with_ns("one", "First")
                      .assert_has_table_with_ns("two", "Second");
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
            namespaces,
            schema_push: SchemaPush::PushAnd(WithSchema::First,
                           &SchemaPush::PushCustomAnd(CustomPushStep {
                           warnings: &[],
                           errors: &[],
                           with_schema: WithSchema::First,
                           executed_steps: ExecutedSteps::Zero,
                           },
                           &SchemaPush::Done)),
            assertion: Box::new(|assert| {
                assert.assert_has_table_with_ns("one", "First")
                      .assert_has_table_with_ns("two", "Second");
            }),
            skip: None,
        },
        TestData {
            name: "mapped table",
            description: "use @map for a model and a field in a namespace",
            schema: Schema {
                common: (base_schema.to_owned() + indoc! {r#" "#}),
                first: indoc! {r#"
                    model First {
                      id Int @id
                      name String @map("name_field")
                      @@map("first_table")
                      @@schema("one")
                    }"#}.into(),
                second: None,
            },
            namespaces,
            schema_push: SchemaPush::PushAnd(WithSchema::First, &SchemaPush::Done),
            assertion: Box::new(|assert| {
                assert.assert_table_with_ns("one", "first_table", |table|
                         table.assert_column("name_field", |column|
                           column.assert_type_is_string()
                         ));
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
                        }
                    "#}.into(),
                ),
            },
            namespaces,
            schema_push: SchemaPush::PushAnd(WithSchema::First, &SchemaPush::PushAnd(WithSchema::Second, &SchemaPush::Done)),
            assertion: Box::new(|assert| {
                assert.assert_has_table_with_ns("one", "First")
                      .assert_has_table_with_ns("two", "Second")
                      .assert_has_table_with_ns("one", "Third");
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
                    } "#
                }.into(),
                second: Some(" ".into()),
            },
            namespaces,
            schema_push: SchemaPush::PushAnd(WithSchema::First, &SchemaPush::PushAnd(WithSchema::Second, &SchemaPush::Done)),
            assertion: Box::new(|assert| {
                assert.assert_has_table_with_ns("one", "First")
                      .assert_has_no_table("Second");
            }),
            skip: None,
        },
        TestData {
            name: "change name of column",
            description: "change the name of a column in a table in a namespace",
            schema: Schema {
                common: (base_schema.to_owned() + indoc! {r#"
                    model First {
                      id Int @id
                      @@schema("one")
                    }"#
                }),
                first: indoc! {r#"
                    model Second {
                      id Int @id
                      name String
                      @@schema("two")
                    }"#}.into(),
                second: Some(indoc!{r#"
                    model Second {
                      id Int @id
                      other_name String
                      @@schema("two")
                    }"#
                }.into()),
            },
            namespaces,
            schema_push: SchemaPush::PushAnd(WithSchema::First,
                           &SchemaPush::PushAnd(WithSchema::Second,
                              &SchemaPush::Done)),
            assertion: Box::new(|assert| {
                assert.assert_has_table_with_ns("one", "First")
                      .assert_table_with_ns("two", "Second", |table|
                          table.assert_column("other_name", |column|
                              column.assert_is_required().assert_type_is_string()
                          ));
            }),
            skip: None,
        },
        TestData {
            name: "add default to column",
            description: "add the @default attribute to a column in an namespace",
            schema: Schema {
                common: (base_schema.to_owned() + indoc! {r#"
                    model First {
                      id Int @id
                      @@schema("one")
                    }"#
                }),
                first: indoc! {r#"
                    model Second {
                      id Int @id
                      name String
                      @@schema("two")
                    }"#
                }.into(),
                second: Some(indoc!{r#"
                    model Second {
                      id Int @id
                      name String @default("hello")
                      @@schema("two")
                    }"#
                }.into()),
            },
            namespaces,
            schema_push: SchemaPush::PushAnd(WithSchema::First,
                           &SchemaPush::PushAnd(WithSchema::Second,
                              &SchemaPush::Done)),
            assertion: Box::new(|assert| {
                assert.assert_has_table_with_ns("one", "First")
                      .assert_table_with_ns("two", "Second", |table|
                          table.assert_column("name", |column|
                              column.assert_is_required()
                                    .assert_type_is_string()
                                    .assert_default(Some(DefaultValue::value("hello")))
                          ));
            }),
            skip: None,
        },
        TestData {
            name: "add autoincrement default to pk",
            description: "add @autoincrement() to a column in a table in a namespace",
            schema: Schema {
                common: (base_schema.to_owned() + indoc! {r#"
                    model First {
                      id Int @id
                      @@schema("one")
                    }"#
                }),
                first: indoc! {r#"
                    model Second {
                      id Int @id
                      @@schema("two")
                    }"#
                }.into(),
                second: Some(indoc!{r#"
                    model Second {
                      id Int @id @default(autoincrement())
                      @@schema("two")
                    }"#
                }.into()),
            },
            namespaces,
            schema_push: SchemaPush::PushAnd(WithSchema::First,
                           &SchemaPush::PushAnd(WithSchema::Second,
                              &SchemaPush::Done)),
            assertion: Box::new(|assert| {
                assert.assert_has_table_with_ns("one", "First")
                      .assert_table_with_ns("two", "Second", |table|
                          table.assert_pk(|pk|
                              pk.assert_has_autoincrement()
                          ));
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
                    }"#
                }),
                first: indoc! {r#"
                    model Second {
                      id Int @id
                      name String?
                      @@schema("two")
                    }"#
                }.into(),
                second: Some(indoc!{r#"
                    model Second {
                      id Int @id
                      name String
                      @@schema("two")
                    }"#
                }.into()),
            },
            namespaces,
            schema_push: SchemaPush::PushAnd(WithSchema::First,
                           &SchemaPush::RawCmdAnd("INSERT INTO \"two\".\"Second\" VALUES(1, 'some value');",
                             &SchemaPush::PushAnd(WithSchema::Second, &SchemaPush::Done))),
            assertion: Box::new(|assert| {
                assert.assert_has_table_with_ns("one", "First")
                      .assert_table_with_ns("two", "Second", |table|
                          table.assert_column("name", |column|
                              column.assert_is_required().assert_type_is_string()
                          ));
            }),
            skip: None,
        },
        TestData {
            name: "rename PK",
            description: "rename a primary key name in a table in a namespace",
            schema: Schema {
                common: (base_schema.to_owned() + indoc! {r#"
                    model First {
                      id Int @id
                      @@schema("one")
                    }"#
                }),
                first: indoc! {r#"
                    model Second {
                      id Int @id
                      @@schema("two")
                    }"#
                }.into(),
                second: Some(indoc!{r#"
                    model Second {
                      new_id_name Int @id
                      @@schema("two")
                    }"#
                }.into()),
            },
            namespaces,
            schema_push: SchemaPush::PushAnd(WithSchema::First,
                           &SchemaPush::PushAnd(WithSchema::Second,
                              &SchemaPush::Done)),
            assertion: Box::new(|assert| {
                assert.assert_has_table_with_ns("one", "First")
                      .assert_table_with_ns("two", "Second", |table|
                          table.assert_pk(|pk|
                            pk.assert_column("new_id_name", |col|
                              col.assert_no_length_prefix()
                          )));
            }),
            skip: None,
        },
        TestData {
            name: "move table across namespaces",
            description: "todo",
            schema: Schema {
                common: (base_schema.to_owned() + indoc! {r#"
        "#}),
                first: indoc! {r#"
        model First {
          id Int @id
          @@schema("one")
        } "#}.into(),
                second: Some(indoc!{r#"
        model First {
          id Int @id
          @@schema("two")
        } "#}.into()),
            },
            namespaces,
            schema_push: SchemaPush::PushAnd(WithSchema::First,
                           &SchemaPush::PushCustomAnd(CustomPushStep {
                               warnings: &[] ,
                               errors: &[],
                               with_schema: WithSchema::Second,
                               executed_steps: ExecutedSteps::NonZero,
                           },
                              &SchemaPush::Done)),
            assertion: Box::new(|assert| {
                assert.assert_has_table_with_ns("two", "First");
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
                    }"#
                }),
                first: indoc! {r#"
                    model Second {
                      id Int @id
                      name String?
                      @@schema("two")
                    }"#
                }.into(),
                second: Some(indoc!{r#"
        model Second {
          id Int @id
          name String
          @@schema("two")
        }"#}.into()),
            },
            namespaces,
            schema_push: SchemaPush::PushAnd(WithSchema::First,
                           &SchemaPush::RawCmdAnd("INSERT INTO \"two\".\"Second\" VALUES(1, NULL);",
                             &SchemaPush::PushCustomAnd(CustomPushStep {
                                 warnings: &[],
                                 errors: &["Made the column `name` on table `Second` required, but there are 1 existing NULL values."],
                                 with_schema: WithSchema::Second,
                                 executed_steps: ExecutedSteps::Zero,
                             },
                             &SchemaPush::Done))),
            assertion: Box::new(|assert| {
                assert
                    .assert_has_table_with_ns("one", "First")
                    .assert_table_with_ns("two", "Second", |table|
                        table.assert_column("name", |column|
                            column.assert_is_nullable()
                        ));
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
                    }"#
                }),
                first: indoc! {r#"
                    model Second {
                      id Int @id
                      @@schema("two")
                    }"#
                }.into(),
                second: Some( indoc! {r#"
                    model Second {
                      id Int @id
                      name String
                      @@schema("two")
                    }"#
                }.into()),
            },
            namespaces,
            schema_push: SchemaPush::PushAnd(WithSchema::First, &SchemaPush::PushAnd(WithSchema::Second, &SchemaPush::Done)),
            assertion: Box::new(|assert| {
                assert
                    .assert_has_table_with_ns("one", "First")
                    .assert_table_with_ns("two", "Second", |table|
                        table.assert_column("name", |column|
                            column.assert_is_required()
                        ));
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
                    }"#
                }),
                first: indoc! {r#"
                    model Second {
                      id Int @id
                      name String
                      @@schema("two")
                    }"#
                }.into(),
                second: Some(indoc! {r#"
                    model Second {
                      id Int @id
                      name String[]
                      @@schema("two")
                    }"#
                }.into()),
            },
            namespaces,
            schema_push: SchemaPush::PushAnd(WithSchema::First, &SchemaPush::PushAnd(WithSchema::Second, &SchemaPush::Done)),
            assertion: Box::new(|assert| {
                assert
                    .assert_has_table_with_ns("one", "First")
                    .assert_table_with_ns("two", "Second", |table|
                        table.assert_column("name", |column|
                            column.assert_is_list()
                        ));
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
                    }"#
                }),
                first: indoc! {r#"
                    model Second {
                      id Int @id
                      name String[]
                      @@schema("two")
                    }"#
                }.into(),
                second: Some(indoc! {r#"
                    model Second {
                      id Int @id
                      name String
                      @@schema("two")
                    }"#
                }.into()),
            },
            namespaces,
            schema_push: SchemaPush::PushAnd(WithSchema::First, &SchemaPush::PushAnd(WithSchema::Second, &SchemaPush::Done)),
            assertion: Box::new(|assert| {
                assert
                    .assert_has_table_with_ns("one", "First")
                    .assert_table_with_ns("two", "Second", |table|
                        table.assert_column("name", |column|
                            column.assert_is_required()
                        ));
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
                    }"#
                }),
                first: indoc! {r#"
                    model Second {
                      id Int @id
                      name String
                      @@index(fields: [name], map: "index_name")
                      @@schema("two")
                    }"#
                }.into(),
                second: Some( indoc! {r#"
                    model Second {
                      id Int @id
                      name String
                      @@index(fields: [name], map: "new_index_name")
                      @@schema("two")
                    }"#
                }.into()),
            },
            namespaces,
            schema_push: SchemaPush::PushAnd(WithSchema::First, &SchemaPush::PushAnd(WithSchema::Second, &SchemaPush::Done)),
            assertion: Box::new(|assert| {
                assert
                    .assert_has_table_with_ns("one", "First")
                    .assert_table_with_ns("two", "Second", |table|
                        table.assert_has_index_name_and_type("new_index_name", false)
                        );
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
                    }"#
                }),
                first: indoc! {r#"
                    model Second {
                      id Int @id
                      name String
                      @@schema("two")
                    }"#
                }.into(),
                second: Some( indoc! {r#"
                    model Second {
                      id Int @id
                      name String @unique
                      @@schema("two")
                    }"#
                }.into()),
            },
            namespaces,
            schema_push: SchemaPush::PushAnd(WithSchema::First,
                           &SchemaPush::PushCustomAnd(CustomPushStep {
                               warnings: &["A unique constraint covering the columns `[name]` on the table `Second` will be added. If there are existing duplicate values, this will fail."] ,
                               errors: &[],
                               with_schema: WithSchema::Second,
                               executed_steps: ExecutedSteps::NonZero,
                           }, &SchemaPush::Done)),
            assertion: Box::new(|assert| {
                assert
                    .assert_has_table_with_ns("one", "First")
                    .assert_table_with_ns("two", "Second", |table|
                        table.assert_index_on_columns(&["name"], |index|
                            index.assert_is_unique()
                        ));
            }),
            skip: None,
        },
        TestData {
            name: "using enums",
            description: "use an enum in a table in a namespace",
            schema: Schema {
                common: (base_schema.to_owned() + indoc! {r#""#}),
                first: indoc! {r#"
                    model First {
                      id Int @id
                      field Second
                      @@schema("one")
                    }

                    enum Second {
                      One
                      Two
                      @@schema("one")
                    }"#
                }.into(),
                second: None,
            },
            namespaces,
            schema_push: SchemaPush::PushAnd(WithSchema::First, &SchemaPush::Done),
            assertion: Box::new(|assert| {
                assert
                    .assert_table_with_ns("one", "First", |table|
                        table.assert_column("field", |column| column.assert_type_is_enum()))
                    .assert_enum("Second", |r#enum|
                        r#enum.assert_namespace("one")
                      );
            }),
            skip: None,
        },
        TestData {
            name: "using enums cross namespaces",
            description: "use enums in a table cross namespace",
            schema: Schema {
                common: (base_schema.to_owned() + indoc! {r#""#}),
                first: indoc! {r#"
                    model First {
                      id Int @id
                      field Second
                      @@schema("one")
                    }

                    enum Second {
                      One
                      Two
                      @@schema("two")
                    }"#
                }.into(),
                second: None,
            },
            namespaces,
            schema_push: SchemaPush::PushAnd(WithSchema::First, &SchemaPush::Done),
            assertion: Box::new(|assert| {
                assert
                    .assert_table_with_ns("one", "First", |table|
                        table.assert_column("field", |column| column.assert_type_is_enum()))
                    .assert_enum("Second", |r#enum| r#enum.assert_namespace("two"));
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
                    }"#
                }),
                first: indoc! {r#"
                    enum Second {
                      One
                      Two
                      @@schema("two")
                    }"#
                }.into(),
                second: Some(indoc! {r#""#}.into()),
            },
            namespaces,
            schema_push: SchemaPush::PushAnd(WithSchema::First, &SchemaPush::PushAnd(WithSchema::Second, &SchemaPush::Done)),
            assertion: Box::new(|assert| {
                assert
                    .assert_has_table_with_ns("one", "First")
                    .assert_has_no_enum("Second");
            }),
            skip: None,
        },
        TestData {
            name: "add a one-to-one relationship cross-namespace relation",
            description: "adds a one-to-one relationship namespace between two tables in different namespaces",
            schema: Schema {
                common: base_schema.to_owned(),
                first: indoc! {r#"
                    model First {
                      id Int @id
                      some_field String
                      @@schema("one")
                    }
                    model Second {
                      id Int @id
                      other_field String
                      @@schema("two")
                    }"#
                }.into(),
                second: Some( indoc! {r#"
                    model First {
                      id Int @id
                      some_field String
                      second Second?
                      @@schema("one")
                    }
                    model Second {
                      id Int @id
                      other_field String
                      first_id Int @unique
                      first First @relation(fields: [first_id], references: [id])
                      @@schema("two")
                    }"#
                }.into()),
            },
            namespaces,
            schema_push: SchemaPush::PushAnd(WithSchema::First,
                           &SchemaPush::PushCustomAnd(CustomPushStep {
                               warnings: &["A unique constraint covering the columns `[first_id]` on the table `Second` will be added. If there are existing duplicate values, this will fail."],
                               errors: &[],
                               with_schema: WithSchema::Second,
                               executed_steps: ExecutedSteps::NonZero,
                           },
                              &SchemaPush::Done)),
            assertion: Box::new(|assert| {
                assert
                    .assert_has_table_with_ns("one", "First")
                    .assert_table_with_ns("two", "Second", |table|
                        table.assert_fk_on_columns(&["first_id"], |fk|
                            fk.assert_references("First", &["id"])
                    ));
            }),
            skip: None,
        },
        TestData {
            name: "drop one-to-one cross-namespace relation",
            description: "drop a one-to-one relationship from tables in different namespaces",
            schema: Schema {
                common: base_schema.to_owned(),
                first: indoc! {r#"
                    model First {
                      id Int @id
                      some_field String
                      second Second?
                      @@schema("one")
                    }
                    model Second {
                      id Int @id
                      other_field String
                      first_id Int @unique
                      first First @relation(fields: [first_id], references: [id])
                      @@schema("two")
                    }"#
                }.into(),
                second: Some( indoc! {r#"
                    model First {
                      id Int @id
                      some_field String
                      @@schema("one")
                    }
                    model Second {
                      id Int @id
                      other_field String
                      @@schema("two")
                    }"#
                }.into()),
            },
            namespaces,
            schema_push: SchemaPush::PushAnd(WithSchema::First,
                           &SchemaPush::PushAnd(WithSchema::Second,
                              &SchemaPush::Done)),
            assertion: Box::new(|assert| {
                assert
                    .assert_table_with_ns("one", "First", |table| table.assert_foreign_keys_count(0))
                    .assert_table_with_ns("two", "Second", |table| table.assert_foreign_keys_count(0));
            }),
            skip: None,
        },
        TestData {
            name: "add one-to-many cross-namespace relation",
            description: "add a one-to-many relationship between tables across namespaces",
            schema: Schema {
                common: base_schema.to_owned(),
                first: indoc! {r#"
                    model First {
                      id Int @id
                      some_field String
                      @@schema("one")
                    }
                    model Second {
                      id Int @id
                      other_field String
                      @@schema("two")
                    }"#
                }.into(),
                second: Some( indoc! {r#"
                    model First {
                      id Int @id
                      some_field String
                      seconds Second[]
                      @@schema("one")
                    }
                    model Second {
                      id Int @id
                      other_field String
                      first_id Int @unique
                      first First @relation(fields: [first_id], references: [id])
                      @@schema("two")
                    }"#
                }.into()),
            },
            namespaces,
            schema_push: SchemaPush::PushAnd(WithSchema::First,
                           &SchemaPush::PushCustomAnd(CustomPushStep {
                               warnings: &["A unique constraint covering the columns `[first_id]` on the table `Second` will be added. If there are existing duplicate values, this will fail."],
                               errors: &[],
                               with_schema: WithSchema::Second,
                               executed_steps: ExecutedSteps::NonZero,
                           },
                              &SchemaPush::Done)),
            assertion: Box::new(|assert| {
                assert
                    .assert_has_table_with_ns("one", "First")
                    .assert_table_with_ns("two", "Second", |table|
                        table.assert_fk_on_columns(&["first_id"], |fk|
                            fk.assert_references("First", &["id"])
                    ));
            }),
            skip: None,
        },
        TestData {
            name: "rename foreign key",
            description: "Rename a foreign key in a table inside a namespace",
            schema: Schema {
                common: base_schema.to_owned(),
                first: indoc! {r#"
        model First {
          id Int @id
          some_field String
          seconds Second[]
          @@schema("one")
        }
        model Second {
          id Int @id
          other_field String
          first_id Int @unique
          first First @relation(fields: [first_id], references: [id])
          @@schema("two")
        }"#}.into(),
                second: Some( indoc! {r#"
        model First {
          id Int @id
          some_field String
          seconds Second[]
          @@schema("one")
        }
        model Second {
          id Int @id
          other_field String
          first_id Int @unique
          first First @relation(fields: [first_id], references: [id], map: "new_name")
          @@schema("two")
        } "#}.into()),
            },
            namespaces,
            schema_push: SchemaPush::PushAnd(WithSchema::First,
                           &SchemaPush::PushAnd(WithSchema::Second,
                              &SchemaPush::Done)),
            assertion: Box::new(|assert| {
                assert
                    .assert_has_table_with_ns("one", "First")
                    .assert_table_with_ns("two", "Second", |table|
                               table.assert_fk_with_name("new_name")
                        );
            }),
            skip: None,
        },
        TestData {
            name: "add explicit many-to-many cross-namespace relation",
            description: "add an explicit many-to-many relationship for tables in different namespaces",
            schema: Schema {
                common: base_schema.to_owned(),
                first: indoc! {r#"
                    model First {
                      id Int @id
                      some_field String
                      @@schema("one")
                    }
                    model Second {
                      id Int @id
                      other_field String
                      @@schema("two")
                    }"#
                }.into(),
                second: Some( indoc! {r#"
                    model First {
                      id Int @id
                      some_field String
                      thirds Third[]
                      @@schema("one")
                    }

                    model Second {
                      id Int @id
                      other_field String
                      thirds Third[]
                      @@schema("two")
                    }

                    model Third {
                      first First @relation(fields: [first_id], references: [id])
                      first_id Int

                      second Second @relation(fields: [second_id], references: [id])
                      second_id Int

                      @@id([first_id, second_id])
                      @@schema("two")
                    }
                    "#
                }.into()),
            },
            namespaces,
            schema_push: SchemaPush::PushAnd(WithSchema::First,
                           &SchemaPush::PushAnd(WithSchema::Second,
                              &SchemaPush::Done)),
            assertion: Box::new(|assert| {
                assert
                    .assert_has_table_with_ns("one", "First")
                    .assert_has_table_with_ns("two", "Second")
                    .assert_table_with_ns("two", "Third", |table| {
                        table.assert_fk_on_columns(&["first_id"], |fk|
                            fk.assert_references("First", &["id"]));

                        table.assert_fk_on_columns(&["second_id"], |fk|
                            fk.assert_references("Second", &["id"]))
                    });
            }),
            skip: None,
        },
        TestData {
            name: "add implicit many-to-many cross-namespace relation",
            description: "add implicit many-to-many relationship between tables in different namespaces ",
            schema: Schema {
                common: base_schema.to_owned(),
                first: indoc! {r#"
                    model First {
                      id Int @id
                      some_field String
                      @@schema("one")
                    }
                    model Second {
                      id Int @id
                      other_field String
                      @@schema("two")
                    }"#
                }.into(),
                second: Some(indoc! {r#"
                    model First {
                      id Int @id
                      some_field String
                      seconds Second[]
                      @@schema("one")
                    }

                    model Second {
                      id Int @id
                      other_field String
                      firsts First[]
                      @@schema("two")
                    }
                    "#
                }.into()),
            },
            namespaces,
            schema_push: SchemaPush::PushAnd(WithSchema::First,
                           &SchemaPush::PushAnd(WithSchema::Second,
                              &SchemaPush::Done)),
            assertion: Box::new(|assert| {
                assert
                    .assert_has_table_with_ns("one", "First")
                    .assert_has_table_with_ns("two", "Second")
                    .assert_table_with_ns("one", "_FirstToSecond", |table| {
                        table.assert_fk_on_columns(&["A"], |fk|
                            fk.assert_references("First", &["id"]));

                        table.assert_fk_on_columns(&["B"], |fk|
                            fk.assert_references("Second", &["id"]))
                    });
            }),
            skip: None,
        },
        TestData {
            name: "add one-to-one self-relation",
            description: "add a one-to-one self relationship in a table in a namespace",
            schema: Schema {
                common: base_schema.to_owned(),
                first: indoc! {r#"
                    model First {
                      id Int @id
                      @@schema("one")
                    }"#
                }.into(),
                second: Some( indoc! {r#"
                    model First {
                      id Int @id
                      next First? @relation("Line", fields: [next_id], references: [id])
                      prev First? @relation("Line")
                      next_id Int? @unique
                      @@schema("one")
                    }"#
                }.into()),
            },
            namespaces,
            schema_push: SchemaPush::PushAnd(WithSchema::First,
                           &SchemaPush::PushCustomAnd(CustomPushStep {
                               warnings: &["A unique constraint covering the columns `[next_id]` on the table `First` will be added. If there are existing duplicate values, this will fail."],
                               errors: &[],
                               with_schema: WithSchema::Second,
                               executed_steps: ExecutedSteps::NonZero,
                           },
                              &SchemaPush::Done)),
            assertion: Box::new(|assert| {
                assert
                    .assert_table_with_ns("one", "First", |table|
                        table.assert_fk_on_columns(&["next_id"], |fk|
                            fk.assert_references("First", &["id"]))
                    );
            }),
            skip: None,
        },
        TestData {
            name: "add one-to-many self-relation",
            description: "add a one-to-many relationnship in a table in a namespace",
            schema: Schema {
                common: base_schema.to_owned(),
                first: indoc! {r#"
                    model First {
                      id Int @id
                      @@schema("one")
                    }"#
                }.into(),
                second: Some( indoc! {r#"
                    model First {
                      id Int @id
                      next First? @relation("Line", fields: [next_id], references: [id])
                      all First[] @relation("Line")
                      next_id Int?
                      @@schema("one")
                    }"#
                }.into()),
            },
            namespaces,
            schema_push: SchemaPush::PushAnd(WithSchema::First,
                           &SchemaPush::PushAnd(WithSchema::Second,
                              &SchemaPush::Done)),
            assertion: Box::new(|assert| {
                assert
                    .assert_table_with_ns("one", "First", |table|
                        table.assert_fk_on_columns(&["next_id"], |fk|
                            fk.assert_references("First", &["id"]))
                    );
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
                    }"#
                }.into(),
                second: Some(indoc! {r#"
                    model First {
                      id Int @id
                      @@schema("one")
                    }
                    model Second {
                      id Int @id
                      @@schema("one")
                    }"#
                }.into()),
            },
            namespaces,
            schema_push: SchemaPush::PushAnd(WithSchema::First, &SchemaPush::PushAnd(WithSchema::Second, &SchemaPush::Done)),
            assertion: Box::new(|assert| {
                assert
                    .assert_has_table_with_ns("one", "First")
                    .assert_table_with_ns("one", "Second", |table|
                            table.assert_column_count(1)
                        );
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
                    }"#
                }),
                first: indoc! {r#"
                    model Second {
                      id Int @id
                      name String
                      @@index(fields: [name], map: "index_name")
                      @@schema("two")
                    }"#
                }.into(),
                second: Some(indoc! {r#"
                    model Second {
                      id Int @id
                      name String
                      @@schema("two")
                    }"#
                }.into()),
            },
            namespaces,
            schema_push: SchemaPush::PushAnd(WithSchema::First, &SchemaPush::PushAnd(WithSchema::Second, &SchemaPush::Done)),
            assertion: Box::new(|assert| {
                assert
                    .assert_has_table_with_ns("one", "First")
                    .assert_table_with_ns("two", "Second", |table|
                        table.assert_indexes_count(0)
                        );
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
                    }"#
                }),
                first: indoc! {r#""#}.into(),
                second: None,
            },
            namespaces,
            schema_push: SchemaPush::PushAnd(WithSchema::First,
                           &SchemaPush::RawCmdAnd("CREATE VIEW \"two\".\"Test\" (id, name) as SELECT id, name FROM \"one\".\"First\"",
                             &SchemaPush::Reset(true,
                               &SchemaPush::Done))),
            assertion: Box::new(|assert| {
                assert.assert_views_count(0);
            }),
            skip: None,
        },
        TestData {
            name: "alter enum",
            description: "Test adding a variant to an enum in a namespace.",
            schema: Schema {
                common: base_schema.to_string(),
                first: indoc! {r#"
                    model SomeModel {
                      id Int @id
                      value SomeEnum @default(First)
                      @@schema("one")
                    }

                    enum SomeEnum {
                      First
                      Second
                      @@schema("one")
                    }"#}.into(),
                              second: Some(indoc! {r#"
                    model SomeModel {
                      id Int @id
                      value SomeEnum @default(First)
                      @@schema("one")
                    }

                    enum SomeEnum {
                      First
                      Third
                      @@schema("one")
                    }"#
                }.into()),
            },
            namespaces,
            schema_push: SchemaPush::PushAnd(WithSchema::First,
                           &SchemaPush::PushCustomAnd(CustomPushStep {
                               warnings: &["The values [Second] on the enum `SomeEnum` will be removed. If these variants are still used in the database, this will fail."],
                               errors: &[],
                               with_schema: WithSchema::Second,
                               executed_steps: ExecutedSteps::NonZero,
                           },
                              &SchemaPush::Done)),
            assertion: Box::new(|assert| {
                assert.assert_has_table("SomeModel")
                      .assert_enum("SomeEnum", |e|
                                         e.assert_values(&["First", "Third"])
                                           .assert_namespace("one"));
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
                    }"#
                }.into(),
                second: Some(indoc! {r#"
                    enum SomeEnum {
                      First
                      Second
                      Three
                      @@schema("two")
                    }"#
                }.into()),
            },
            namespaces,
            schema_push: SchemaPush::PushAnd(WithSchema::First, &SchemaPush::PushAnd(WithSchema::Second, &SchemaPush::Done)),
            assertion: Box::new(|assert| {
                assert.assert_enum("SomeEnum", |e|
                                         e.assert_values(&["First", "Second", "Three"])
                                          .assert_namespace("two"));
            }),
            skip: None,
        },
        TestData {
            name: "modify and move enum across namespaces",
            description: "this is a special case of alter enum, where we also move it across namespaces",
            schema: Schema {
                common: base_schema.to_string(),
                first: indoc! {r#"
      model SomeModel {
        id Int @id
        value SomeEnum @default(First)
        @@schema("one")
      }

      enum SomeEnum {
        First
        Second
        @@schema("one")
      }"#}.into(),
                second: Some(indoc! {r#"
      model SomeModel {
        id Int @id
        value SomeEnum @default(First)
        @@schema("one")
      }

      enum SomeEnum {
        First
        Third
        @@schema("two")
      }"#}.into()),
            },
            namespaces,
            schema_push: SchemaPush::PushAnd(WithSchema::First,
                           &SchemaPush::RawCmdAnd("insert into \"one\".\"SomeModel\" values(1, 'First');",
                             &SchemaPush::PushCustomAnd(CustomPushStep {
                                 warnings: &["The `value` column on the `SomeModel` table would be dropped and recreated. This will lead to data loss."],
                                 errors: &[],
                                 with_schema: WithSchema::Second,
                                 executed_steps: ExecutedSteps::NonZero,
                             },
                                &SchemaPush::Done))),
            assertion: Box::new(|assert| {
                assert.assert_has_table("SomeModel")
                      .assert_enum("SomeEnum", |e|
                                         e.assert_values(&["First", "Third"])
                                           .assert_namespace("two"));
            }),
            skip: None,
        },
    ];

    // traverse_ is always the answer
    tests.iter_mut().filter(|t| t.skip.is_none()).for_each(|t| {
        run_test(t);
    });
}

#[tokio::test]
async fn migration_with_shadow_database() {
    let conn_str = std::env::var("TEST_DATABASE_URL").unwrap();

    let is_cockroach = conn_str.contains("localhost:2625") || conn_str.contains("localhost:26260");
    if !conn_str.starts_with("postgres") || is_cockroach {
        return;
    }

    let (params, datasource) = {
        let mut shadow_str: Url = conn_str.parse().unwrap();
        shadow_str.set_path("shadow");

        let shadow_str = shadow_str.to_string();

        let datasource = formatdoc! {r#"
            datasource db {{
              provider          = "postgresql"
              url               = "{conn_str}"
              shadowDatabaseUrl = "{shadow_str}"
              schemas           = ["one", "two"]
            }}

            generator js {{
              provider        = "prisma-client-javascript"
              previewFeatures = ["multiSchema"]
            }}
        "#};

        let params = ConnectorParams {
            connection_string: dbg!(conn_str),
            preview_features: PreviewFeature::MultiSchema.into(),
            shadow_database_connection_string: dbg!(Some(shadow_str)),
        };

        (params, datasource)
    };

    let namespaces = Namespaces::from_vec(&mut vec![String::from("dbo"), String::from("one"), String::from("two")]);

    let mut conn = {
        let mut conn = SqlSchemaConnector::new_postgres();

        conn.set_params(params).unwrap();
        let _ = conn.raw_cmd("DROP DATABASE shadow").await;

        conn.raw_cmd("CREATE DATABASE shadow").await.unwrap();
        conn.reset(false, namespaces.clone()).await.unwrap();

        let _ = conn.raw_cmd("DROP SCHEMA one CASCADE").await;
        let _ = conn.raw_cmd("DROP SCHEMA two CASCADE").await;
        let _ = conn.raw_cmd("DROP SCHEMA public CASCADE").await;
        let _ = conn.raw_cmd("CREATE SCHEMA public").await;

        conn
    };

    let dm = formatdoc! {r#"
        {datasource}

        model A {{
          id  Int @id
          bId Int
          bs  B[] @relation("one")
          b   B   @relation("two", fields: [bId], references: [id], onDelete: NoAction, onUpdate: NoAction)

          @@schema("one")
        }}

        model B {{
          id  Int @id
          aId Int
          a   A   @relation("one", fields: [aId], references: [id])
          as  A[] @relation("two")

          @@schema("two")
        }}
    "#};

    let migrations_directory = tempfile::tempdir().unwrap();

    let migration = CreateMigrationInput {
        migrations_directory_path: migrations_directory.path().to_str().unwrap().to_owned(),
        schema: SchemasContainer {
            files: vec![SchemaContainer {
                path: "schema.prisma".to_string(),
                content: dm.clone(),
            }],
        },
        draft: false,
        migration_name: "init".to_string(),
    };

    create_migration(migration, &mut conn).await.unwrap();

    let path = std::fs::read_dir(migrations_directory.path())
        .expect("Reading migrations directory for named migration.")
        .find_map(|entry| {
            let entry = entry.unwrap();
            let name = entry.file_name();

            if name.to_str().unwrap().contains("init") {
                Some(entry)
            } else {
                None
            }
        })
        .unwrap()
        .path()
        .join("migration.sql");

    let sql = std::fs::read_to_string(path).unwrap();

    let expected = expect![[r#"
        -- CreateSchema
        CREATE SCHEMA IF NOT EXISTS "one";

        -- CreateSchema
        CREATE SCHEMA IF NOT EXISTS "two";

        -- CreateTable
        CREATE TABLE "one"."A" (
            "id" INTEGER NOT NULL,
            "bId" INTEGER NOT NULL,

            CONSTRAINT "A_pkey" PRIMARY KEY ("id")
        );

        -- CreateTable
        CREATE TABLE "two"."B" (
            "id" INTEGER NOT NULL,
            "aId" INTEGER NOT NULL,

            CONSTRAINT "B_pkey" PRIMARY KEY ("id")
        );

        -- AddForeignKey
        ALTER TABLE "one"."A" ADD CONSTRAINT "A_bId_fkey" FOREIGN KEY ("bId") REFERENCES "two"."B"("id") ON DELETE NO ACTION ON UPDATE NO ACTION;

        -- AddForeignKey
        ALTER TABLE "two"."B" ADD CONSTRAINT "B_aId_fkey" FOREIGN KEY ("aId") REFERENCES "one"."A"("id") ON DELETE RESTRICT ON UPDATE CASCADE;
    "#]];

    expected.assert_eq(&sql);

    let input = ApplyMigrationsInput {
        migrations_directory_path: migrations_directory.path().to_str().unwrap().to_owned(),
    };

    apply_migrations(input, &mut conn, namespaces).await.unwrap();
}
