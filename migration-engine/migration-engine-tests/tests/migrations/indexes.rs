use migration_engine_tests::sync_test_api::*;

#[test_connector]
fn index_on_compound_relation_fields_must_work(api: TestApi) {
    let dm = r#"
        model User {
            id String @id
            email String
            name String
            p    Post[]

            @@unique([email, name])
        }

        model Post {
            id String @id
            authorEmail String
            authorName String
            author User @relation(fields: [authorEmail, authorName], references: [email, name])

            @@index([authorEmail, authorName], name: "testIndex")
        }
    "#;

    api.schema_push(dm).send_sync().assert_green_bang();

    api.assert_schema().assert_table_bang("Post", |table| {
        table
            .assert_has_column("authorName")?
            .assert_has_column("authorEmail")?
            .assert_index_on_columns(&["authorEmail", "authorName"], |idx| idx.assert_name("testIndex"))
    });
}

#[test_connector]
fn index_settings_must_be_migrated(api: TestApi) {
    let dm = r#"
        model Test {
            id String @id
            name String
            followersCount Int

            @@index([name, followersCount], name: "nameAndFollowers")
        }
    "#;

    api.schema_push(dm).send_sync().assert_green_bang();

    api.assert_schema().assert_table_bang("Test", |table| {
        table
            .assert_indexes_count(1)?
            .assert_index_on_columns(&["name", "followersCount"], |idx| {
                idx.assert_is_not_unique()?.assert_name("nameAndFollowers")
            })
    });

    let dm2 = r#"
        model Test {
            id String @id
            name String
            followersCount Int

            @@unique([name, followersCount], name: "nameAndFollowers")
        }
    "#;

    api.schema_push(dm2)
        .force(true)
        .send_sync()
        .assert_warnings(&["A unique constraint covering the columns `[name,followersCount]` on the table `Test` will be added. If there are existing duplicate values, this will fail.".into()]);

    api.assert_schema().assert_table_bang("Test", |table| {
        table
            .assert_indexes_count(1)?
            .assert_index_on_columns(&["name", "followersCount"], |idx| {
                idx.assert_is_unique()?.assert_name("nameAndFollowers")
            })
    });
}

#[test_connector]
fn unique_directive_on_required_one_to_one_relation_creates_one_index(api: TestApi) {
    // We want to test that only one index is created, because of the implicit unique index on
    // required 1:1 relations.

    let dm = r#"
        model Cat {
            id    Int @id
            boxId Int @unique
            box   Box @relation(fields: [boxId], references: [id])
        }

        model Box {
            id  Int  @id
            cat Cat?
        }
    "#;

    api.schema_push(dm).send_sync().assert_green_bang();

    api.assert_schema()
        .assert_table_bang("Cat", |table| table.assert_indexes_count(1));
}

// TODO: Enable SQL Server when cascading rules are in PSL.
#[test_connector(exclude(Mssql))]
fn one_to_many_self_relations_do_not_create_a_unique_index(api: TestApi) {
    let dm = r#"
        model Location {
            id        String      @id @default(cuid())
            parent    Location?   @relation("LocationToLocation_parent", fields:[parentId], references: [id])
            parentId  String?     @map("parent")
            children  Location[]  @relation("LocationToLocation_parent")
        }
    "#;

    api.schema_push(dm).send_sync().assert_green_bang();

    if api.is_mysql() {
        // MySQL creates an index for the FK.
        api.assert_schema().assert_table_bang("Location", |t| {
            t.assert_indexes_count(1)?
                .assert_index_on_columns(&["parent"], |idx| idx.assert_is_not_unique())
        });
    } else {
        api.assert_schema()
            .assert_table_bang("Location", |t| t.assert_indexes_count(0));
    }
}

#[test_connector]
fn model_with_multiple_indexes_works(api: TestApi) {
    let dm = r#"
    model User {
      id         Int       @id
      l          Like[]
    }

    model Post {
      id        Int       @id
      l         Like[]
    }

    model Comment {
      id        Int       @id
      l         Like[]
    }

    model Like {
      id        Int       @id
      user_id   Int
      user      User @relation(fields: [user_id], references: [id])
      post_id   Int
      post      Post @relation(fields: [post_id], references: [id])
      comment_id Int
      comment   Comment @relation(fields: [comment_id], references: [id])

      @@index([post_id])
      @@index([user_id])
      @@index([comment_id])
    }
    "#;

    api.schema_push(dm).send_sync().assert_green_bang();
    api.assert_schema()
        .assert_table_bang("Like", |table| table.assert_indexes_count(3));
}

#[test_connector]
fn removing_multi_field_unique_index_must_work(api: TestApi) {
    let dm1 = r#"
        model A {
            id    Int    @id
            field String
            secondField Int

            @@unique([field, secondField])
        }
    "#;

    api.schema_push(dm1).send_sync().assert_green_bang();

    api.assert_schema().assert_table_bang("A", |table| {
        table.assert_index_on_columns(&["field", "secondField"], |idx| idx.assert_is_unique())
    });

    let dm2 = r#"
        model A {
            id    Int    @id
            field String
            secondField Int
        }
    "#;

    api.schema_push(dm2).send_sync().assert_green_bang();

    api.assert_schema()
        .assert_table_bang("A", |table| table.assert_indexes_count(0));
}

#[test_connector]
fn index_renaming_must_work(api: TestApi) {
    let dm1 = r#"
        model A {
            id Int @id
            field String
            secondField Int

            @@unique([field, secondField], name: "customName")
            @@index([secondField, field], name: "customNameNonUnique")
        }
    "#;

    api.schema_push(dm1).send_sync().assert_green_bang();

    api.assert_schema().assert_table_bang("A", |table| {
        table
            .assert_index_on_columns(&["field", "secondField"], |idx| {
                idx.assert_is_unique()?.assert_name("customName")
            })?
            .assert_index_on_columns(&["secondField", "field"], |idx| {
                idx.assert_is_not_unique()?.assert_name("customNameNonUnique")
            })
    });

    let dm2 = r#"
        model A {
            id Int @id
            field String
            secondField Int

            @@unique([field, secondField], name: "customNameA")
            @@index([secondField, field], name: "customNameNonUniqueA")
        }
    "#;

    api.schema_push(dm2).send_sync().assert_green_bang();

    api.assert_schema().assert_table_bang("A", |table| {
        table
            .assert_indexes_count(2)?
            .assert_index_on_columns(&["field", "secondField"], |idx| {
                idx.assert_is_unique()?.assert_name("customNameA")
            })?
            .assert_index_on_columns(&["secondField", "field"], |idx| {
                idx.assert_is_not_unique()?.assert_name("customNameNonUniqueA")
            })
    });
}

#[test_connector]
fn index_renaming_must_work_when_renaming_to_default(api: TestApi) {
    let dm1 = r#"
        model A {
            id Int @id
            field String
            secondField Int

            @@unique([field, secondField], name: "customName")
        }
    "#;

    api.schema_push(dm1).send_sync().assert_green_bang();
    api.assert_schema().assert_table_bang("A", |t| {
        t.assert_index_on_columns(&["field", "secondField"], |idx| idx.assert_is_unique())
    });

    let dm2 = r#"
        model A {
            id Int @id
            field String
            secondField Int

            @@unique([field, secondField])
        }
    "#;

    api.schema_push(dm2).send_sync();
    api.assert_schema().assert_table_bang("A", |t| {
        t.assert_index_on_columns(&["field", "secondField"], |idx| {
            idx.assert_is_unique()?.assert_name("A.field_secondField_unique")
        })
    });
}

#[test_connector]
fn index_renaming_must_work_when_renaming_to_custom(api: TestApi) {
    let dm1 = r#"
        model A {
            id Int @id
            field String
            secondField Int

            @@unique([field, secondField])
        }
    "#;

    api.schema_push(dm1).send_sync().assert_green_bang();

    api.assert_schema().assert_table_bang("A", |table| {
        table
            .assert_indexes_count(1)?
            .assert_index_on_columns(&["field", "secondField"], |idx| idx.assert_is_unique())
    });

    let dm2 = r#"
        model A {
            id Int @id
            field String
            secondField Int

            @@unique([field, secondField], name: "somethingCustom")
        }
    "#;

    api.schema_push(dm2).send_sync().assert_green_bang();

    api.assert_schema().assert_table_bang("A", |table| {
        table
            .assert_indexes_count(1)?
            .assert_index_on_columns(&["field", "secondField"], |idx| {
                idx.assert_name("somethingCustom")?.assert_is_unique()
            })
    });
}

#[test_connector]
fn index_updates_with_rename_must_work(api: TestApi) {
    let dm1 = r#"
        model A {
            id Int @id
            field String
            secondField Int

            @@unique([field, secondField], name: "customName")
        }
    "#;

    api.schema_push(dm1).send_sync().assert_green_bang();
    api.assert_schema().assert_table_bang("A", |t| {
        t.assert_index_on_columns(&["field", "secondField"], |idx| idx.assert_is_unique())
    });

    let dm2 = r#"
        model A {
            id Int @id
            field String
            secondField Int

            @@unique([field, id], name: "customNameA")
        }
    "#;

    api.schema_push(dm2).force(true).send_sync().assert_executable();

    api.assert_schema().assert_table_bang("A", |t| {
        t.assert_indexes_count(1)?.assert_index_on_columns(&["field", "id"], Ok)
    });
}

#[test_connector]
fn dropping_a_model_with_a_multi_field_unique_index_must_work(api: TestApi) {
    let dm1 = r#"
        model A {
            id Int @id
            field String
            secondField Int

            @@unique([field, secondField], name: "customName")
        }
    "#;

    api.schema_push(dm1).send_sync().assert_green_bang();
    api.assert_schema().assert_table_bang("A", |t| {
        t.assert_index_on_columns(&["field", "secondField"], |idx| {
            idx.assert_name("customName")?.assert_is_unique()
        })
    });

    api.schema_push("").send_sync().assert_green_bang();
}

#[test_connector(tags(Postgres, Mysql))]
fn indexes_with_an_automatically_truncated_name_are_idempotent(api: TestApi) {
    let dm = r#"
        model TestModelWithALongName {
            id Int @id
            looooooooooooongfield String
            evenLongerFieldNameWth String
            omgWhatEvenIsThatLongFieldName String

            @@index([looooooooooooongfield, evenLongerFieldNameWth, omgWhatEvenIsThatLongFieldName])
        }
    "#;

    api.schema_push(dm).send_sync().assert_green_bang();

    api.assert_schema()
        .assert_table_bang("TestModelWithALongName", |table| {
            table.assert_index_on_columns(
                &[
                    "looooooooooooongfield",
                    "evenLongerFieldNameWth",
                    "omgWhatEvenIsThatLongFieldName",
                ],
                |idx| {
                    idx.assert_name(if api.is_mysql() {
                        // The size limit of identifiers is 64 bytes on MySQL
                        // and 63 on Postgres.
                        "TestModelWithALongName.looooooooooooongfield_evenLongerFieldName"
                    } else {
                        "TestModelWithALongName.looooooooooooongfield_evenLongerFieldNam"
                    })
                },
            )
        });

    api.schema_push(dm).send_sync().assert_green_bang().assert_no_steps();
}

#[test_connector]
fn new_index_with_same_name_as_index_from_dropped_table_works(api: TestApi) {
    let dm1 = r#"
        model Cat {
            id Int @id
            ownerid String
            owner Owner @relation(fields: [ownerid], references: id)

            @@index([ownerid])
        }

        model Other {
            id Int @id
            ownerid String

            @@index([ownerid], name: "ownerid")
        }

        model Owner {
            id String @id
            c  Cat[]
        }
    "#;

    api.schema_push(dm1).send_sync().assert_green_bang();

    api.assert_schema().assert_table_bang("Cat", |table| {
        table.assert_column("ownerid", |col| col.assert_is_required())
    });

    let dm2 = r#"
        model Owner {
            id      Int @id
            ownerid String
            owner   Cat @relation(fields: [ownerid], references: id)

            @@index([ownerid], name: "ownerid")
        }

        model Cat {
            id      String @id
            owners  Owner[]
        }
    "#;

    api.schema_push(dm2).send_sync().assert_green_bang();

    api.assert_schema().assert_table_bang("Owner", |table| {
        table.assert_column("ownerid", |col| col.assert_is_required())
    });
}

#[test_connector]
fn column_type_migrations_should_not_implicitly_drop_indexes(api: TestApi) {
    let migrations_directory = api.create_migrations_directory();

    let dm1 = r#"
        model Cat {
            id Int @id @default(autoincrement())
            name String

            @@index([name])
        }
    "#;

    api.create_migration("01init", dm1, &migrations_directory).send_sync();

    let dm2 = r#"
        model Cat {
            id Int @id
            name Int

            @@index([name])
        }
    "#;

    // NOTE: we are relying on the fact that we will drop and recreate the column for that particular type migration.
    api.create_migration("02change", dm2, &migrations_directory).send_sync();

    api.apply_migrations(&migrations_directory).send_sync();

    api.assert_schema().assert_table_bang("Cat", |cat| {
        cat.assert_indexes_count(1)?
            .assert_index_on_columns(&["name"], |idx| idx.assert_is_not_unique())
    });
}

#[test_connector]
fn column_type_migrations_should_not_implicitly_drop_compound_indexes(api: TestApi) {
    let migrations_directory = api.create_migrations_directory();

    let dm1 = r#"
        model Cat {
            id Int @id @default(autoincrement())
            name String
            age Int

            @@index([name, age])
        }
    "#;

    api.create_migration("01init", dm1, &migrations_directory).send_sync();

    let dm2 = r#"
        model Cat {
            id Int @id
            name Int
            age Int

            @@index([name, age])
        }
    "#;

    // NOTE: we are relying on the fact that we will drop and recreate the column for that particular type migration.
    api.create_migration("02change", dm2, &migrations_directory).send_sync();

    api.apply_migrations(&migrations_directory).send_sync();

    api.assert_schema().assert_table_bang("Cat", |cat| {
        cat.assert_indexes_count(1)?
            .assert_index_on_columns(&["name", "age"], |idx| idx.assert_is_not_unique())
    });
}
