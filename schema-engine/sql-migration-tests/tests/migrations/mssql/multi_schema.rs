use crate::migrations::multi_schema::*;
use connection_string::JdbcString;
use indoc::{formatdoc, indoc};
use sql_migration_tests::test_api::*;
use sql_schema_describer::DefaultValue;

// This is the only "top" level test in this module. It defines a list of tests and executes them.
// If you want to look at the tests, see the `tests` variable below.
#[test_connector(tags(Mssql, Mssql2019, Mssql2017), namespaces("one", "two"))]
fn multi_schema_tests(_api: TestApi) {
    let namespaces: &'static [&'static str] = &["one", "two"];
    let base_schema = indoc! {r#"
        datasource db {
          provider   = "sqlserver"
          url        = env("TEST_DATABASE_URL")
          schemas    = ["one", "two"]
        }

        generator js {
          provider = "prisma-client-js"
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
                    } "#}
                .into(),
                second: None,
            },
            namespaces,
            schema_push: SchemaPush::PushAnd(WithSchema::First, &SchemaPush::Done),
            assertion: Box::new(|assert| {
                assert
                    .assert_has_table_with_ns("one", "First")
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
                    }"#}
                .into(),
                second: None,
            },
            namespaces,
            schema_push: SchemaPush::PushAnd(
                WithSchema::First,
                &SchemaPush::PushCustomAnd(
                    CustomPushStep {
                        warnings: &[],
                        errors: &[],
                        with_schema: WithSchema::First,
                        executed_steps: ExecutedSteps::Zero,
                    },
                    &SchemaPush::Done,
                ),
            ),
            assertion: Box::new(|assert| {
                assert
                    .assert_has_table_with_ns("one", "First")
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
                    }"#}
                .into(),
                second: None,
            },
            namespaces,
            schema_push: SchemaPush::PushAnd(WithSchema::First, &SchemaPush::Done),
            assertion: Box::new(|assert| {
                assert.assert_table_with_ns("one", "first_table", |table| {
                    table.assert_column("name_field", |column| column.assert_type_is_string())
                });
            }),
            skip: None,
        },
        TestData {
            name: "add table",
            description: "Test adding a new table to one of the namespaces",
            schema: Schema {
                common: (base_schema.to_owned()
                    + indoc! {r#"
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
                    "#}
                    .into(),
                ),
            },
            namespaces,
            schema_push: SchemaPush::PushAnd(
                WithSchema::First,
                &SchemaPush::PushAnd(WithSchema::Second, &SchemaPush::Done),
            ),
            assertion: Box::new(|assert| {
                assert
                    .assert_has_table_with_ns("one", "First")
                    .assert_has_table_with_ns("two", "Second")
                    .assert_has_table_with_ns("one", "Third");
            }),
            skip: None,
        },
        TestData {
            name: "remove table",
            description: "Test removing a table to one of the namespaces",
            schema: Schema {
                common: (base_schema.to_owned()
                    + indoc! {r#"
                    model First {
                      id Int @id
                      @@schema("one")
                    }"#}),
                first: indoc! {r#"
                    model Second {
                      id Int @id
                      @@schema("two")
                    } "#
                }
                .into(),
                second: Some(" ".into()),
            },
            namespaces,
            schema_push: SchemaPush::PushAnd(
                WithSchema::First,
                &SchemaPush::PushAnd(WithSchema::Second, &SchemaPush::Done),
            ),
            assertion: Box::new(|assert| {
                assert
                    .assert_has_table_with_ns("one", "First")
                    .assert_has_no_table("Second");
            }),
            skip: None,
        },
        TestData {
            name: "change name of column",
            description: "change the name of a column in a table in a namespace",
            schema: Schema {
                common: (base_schema.to_owned()
                    + indoc! {r#"
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
                    }"#}
                .into(),
                second: Some(
                    indoc! {r#"
                    model Second {
                      id Int @id
                      other_name String
                      @@schema("two")
                    }"#
                    }
                    .into(),
                ),
            },
            namespaces,
            schema_push: SchemaPush::PushAnd(
                WithSchema::First,
                &SchemaPush::PushAnd(WithSchema::Second, &SchemaPush::Done),
            ),
            assertion: Box::new(|assert| {
                assert
                    .assert_has_table_with_ns("one", "First")
                    .assert_table_with_ns("two", "Second", |table| {
                        table.assert_column("other_name", |column| {
                            column.assert_is_required().assert_type_is_string()
                        })
                    });
            }),
            skip: None,
        },
        TestData {
            name: "add default to column",
            description: "add the @default attribute to a column in an namespace",
            schema: Schema {
                common: (base_schema.to_owned()
                    + indoc! {r#"
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
                }
                .into(),
                second: Some(
                    indoc! {r#"
                    model Second {
                      id Int @id
                      name String @default("hello")
                      @@schema("two")
                    }"#
                    }
                    .into(),
                ),
            },
            namespaces,
            schema_push: SchemaPush::PushAnd(
                WithSchema::First,
                &SchemaPush::PushAnd(WithSchema::Second, &SchemaPush::Done),
            ),
            assertion: Box::new(|assert| {
                assert
                    .assert_has_table_with_ns("one", "First")
                    .assert_table_with_ns("two", "Second", |table| {
                        table.assert_column("name", |column| {
                            let mut default = DefaultValue::value("hello");
                            default.set_constraint_name("Second_name_df");
                            column
                                .assert_is_required()
                                .assert_type_is_string()
                                .assert_default(Some(default))
                        })
                    });
            }),
            skip: None,
        },
        TestData {
            name: "add autoincrement default to pk",
            description: "add @autoincrement() to a column in a table in a namespace",
            schema: Schema {
                common: (base_schema.to_owned()
                    + indoc! {r#"
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
                }
                .into(),
                second: Some(
                    indoc! {r#"
                    model Second {
                      id Int @id @default(autoincrement())
                      @@schema("two")
                    }"#
                    }
                    .into(),
                ),
            },
            namespaces,
            schema_push: SchemaPush::PushAnd(
                WithSchema::First,
                &SchemaPush::PushAnd(WithSchema::Second, &SchemaPush::Done),
            ),
            assertion: Box::new(|assert| {
                assert
                    .assert_has_table_with_ns("one", "First")
                    .assert_table_with_ns("two", "Second", |table| {
                        table.assert_pk(|pk| pk.assert_has_autoincrement())
                    });
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
                      next First? @relation("Line", fields: [next_id], references: [id], onDelete: NoAction, onUpdate: NoAction)
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
                      next First? @relation("Line", fields: [next_id], references: [id], onUpdate: NoAction, onDelete: NoAction)
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
                    }
                    model Second {
                      id Int @id
                      name String
                      @@schema("two")
                    }"#
                }),
                first: indoc! {r#""#}.into(),
                second: None,
            },
            namespaces,
            schema_push: SchemaPush::PushAnd(WithSchema::First,
                           &SchemaPush::RawCmdAnd("CREATE VIEW [two].[Test] (id, name) as SELECT id, name FROM [one].[First]",
                             &SchemaPush::Reset(true,
                               &SchemaPush::Done))),
            assertion: Box::new(|assert| {
                assert.assert_views_count(0);
            }),
            skip: None,
        },
        TestData {
            name: "issue prisma/prisma#24068",
            description: "Modify the primary key of a table in a namespace",
            schema: Schema {
                common: base_schema.into(),
                first: indoc! {r#"
                    model Report {
                      id String @id

                      @@schema("two")
                    }"#}
                .into(),
                second: Some(indoc! {r#"
                    model Report {
                      id String @id @unique @db.NVarChar(450)

                      @@schema("two")
                    }"#}
                .into())
            },
            namespaces,
            schema_push: SchemaPush::PushAnd(WithSchema::First, &SchemaPush::PushCustomAnd(CustomPushStep {
                warnings: &["A unique constraint covering the columns `[id]` on the table `Report` will be added. If there are existing duplicate values, this will fail."],
                errors: &[],
                with_schema: WithSchema::Second,
                executed_steps: ExecutedSteps::NonZero,
            }, &SchemaPush::Done)),
            assertion: Box::new(|assert| {
                assert
                    .assert_has_table_with_ns("two", "Report")
                    .assert_table_with_ns("two", "Report", |table| {
                        table.assert_column("id", |column| column.assert_is_required().assert_type_is_string())
                    });
            }),
            skip: None,
        },
    ];

    // traverse_ is always the answer
    tests.iter_mut().filter(|t| t.skip.is_none()).for_each(|t| {
        println!("Running test: {}", t.name);
        run_test(t);
    });
}

#[test_connector(tags(Mssql), namespaces("one", "two"))]
fn multi_schema_migration(api: TestApi) {
    let dm = indoc! {r#"
        datasource db {
          provider = "sqlserver"
          url      = env("TEST_DATABASE_URL")
          schemas  = ["one", "two"]
        }

        generator js {
          provider = "prisma-client-js"
        }

        model A {
          id  Int @id
          bId Int
          bs  B[] @relation("one")
          b   B   @relation("two", fields: [bId], references: [id], onDelete: NoAction, onUpdate: NoAction)

          @@schema("one")
        }

        model B {
          id  Int @id
          aId Int
          a   A   @relation("one", fields: [aId], references: [id])
          as  A[] @relation("two")

          @@schema("two")
        }
    "#};

    let dir = api.create_migrations_directory();

    api.create_migration("init", dm, &dir).send_sync();

    api.apply_migrations(&dir)
        .send_sync()
        .assert_applied_migrations(&["init"]);

    api.apply_migrations(&dir).send_sync().assert_applied_migrations(&[]);
}

#[test_connector(tags(Mssql), namespaces("one", "two"))]
fn migration_with_shadow_database(api: TestApi) {
    let conn_str = std::env::var("TEST_DATABASE_URL").unwrap();

    let mut shadow_str: JdbcString = format!("jdbc:{conn_str}").parse().unwrap();

    shadow_str
        .properties_mut()
        .insert("database".to_string(), "shadow".to_string());

    let shadow_str = shadow_str.to_string().replace("jdbc:", "");

    let datasource = formatdoc! {r#"
        datasource db {{
          provider          = "sqlserver"
          url               = "{conn_str}"
          shadowDatabaseUrl = "{shadow_str}"
          schemas           = ["one", "two"]
        }}

        generator js {{
          provider        = "prisma-client-javascript"
        }}
    "#};

    let namespaces = Namespaces::from_vec(&mut vec![String::from("dbo"), String::from("one"), String::from("two")]);

    api.raw_cmd("DROP DATABASE IF EXISTS shadow");
    api.raw_cmd("CREATE DATABASE shadow");
    api.reset().send_sync(namespaces.clone());

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

    let dir = api.create_migrations_directory();

    api.create_migration("init", &dm, &dir)
        .send_sync()
        .assert_migration_directories_count(1)
        .assert_migration("init", move |migration| {
            let expected_script = expect![[r#"
                BEGIN TRY

                BEGIN TRAN;

                -- CreateSchema
                IF NOT EXISTS (SELECT * FROM sys.schemas WHERE name = N'one') EXEC sp_executesql N'CREATE SCHEMA [one];';

                -- CreateSchema
                IF NOT EXISTS (SELECT * FROM sys.schemas WHERE name = N'two') EXEC sp_executesql N'CREATE SCHEMA [two];';

                -- CreateTable
                CREATE TABLE [one].[A] (
                    [id] INT NOT NULL,
                    [bId] INT NOT NULL,
                    CONSTRAINT [A_pkey] PRIMARY KEY CLUSTERED ([id])
                );

                -- CreateTable
                CREATE TABLE [two].[B] (
                    [id] INT NOT NULL,
                    [aId] INT NOT NULL,
                    CONSTRAINT [B_pkey] PRIMARY KEY CLUSTERED ([id])
                );

                -- AddForeignKey
                ALTER TABLE [one].[A] ADD CONSTRAINT [A_bId_fkey] FOREIGN KEY ([bId]) REFERENCES [two].[B]([id]) ON DELETE NO ACTION ON UPDATE NO ACTION;

                -- AddForeignKey
                ALTER TABLE [two].[B] ADD CONSTRAINT [B_aId_fkey] FOREIGN KEY ([aId]) REFERENCES [one].[A]([id]) ON DELETE NO ACTION ON UPDATE CASCADE;

                COMMIT TRAN;

                END TRY
                BEGIN CATCH

                IF @@TRANCOUNT > 0
                BEGIN
                    ROLLBACK TRAN;
                END;
                THROW

                END CATCH
            "#]];

            migration.expect_contents(expected_script)
        });

    api.apply_migrations(&dir)
        .send_sync()
        .assert_applied_migrations(&["init"]);
}
