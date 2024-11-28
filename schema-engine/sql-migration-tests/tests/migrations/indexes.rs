mod cockroachdb;
mod mssql;
mod postgres;

use indoc::{formatdoc, indoc};
use sql_migration_tests::test_api::*;
use sql_schema_describer::SQLSortOrder;

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

    api.schema_push_w_datasource(dm).send().assert_green();

    api.assert_schema().assert_table("Post", |table| {
        table
            .assert_has_column("authorName")
            .assert_has_column("authorEmail")
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

            @@index([name, followersCount], map: "nameAndFollowers")
        }
    "#;

    api.schema_push_w_datasource(dm).send().assert_green();

    api.assert_schema().assert_table("Test", |table| {
        table
            .assert_indexes_count(1)
            .assert_index_on_columns(&["name", "followersCount"], |idx| {
                idx.assert_is_not_unique().assert_name("nameAndFollowers")
            })
    });

    let dm2 = r#"
        model Test {
            id String @id
            name String
            followersCount Int

            @@unique([name, followersCount], map: "nameAndFollowers")
        }
    "#;

    api.schema_push_w_datasource(dm2)
        .force(true)
        .send()
        .assert_warnings(&["A unique constraint covering the columns `[name,followersCount]` on the table `Test` will be added. If there are existing duplicate values, this will fail.".into()]);

    api.assert_schema().assert_table("Test", |table| {
        table
            .assert_indexes_count(1)
            .assert_index_on_columns(&["name", "followersCount"], |idx| {
                idx.assert_is_unique().assert_name("nameAndFollowers")
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

    api.schema_push_w_datasource(dm).send().assert_green();

    api.assert_schema()
        .assert_table("Cat", |table| table.assert_indexes_count(1));
}

#[test_connector(exclude(Vitess))]
fn one_to_many_self_relations_do_not_create_a_unique_index(api: TestApi) {
    let dm = r#"
        model Location {
            id        String      @id @default(cuid())
            parent    Location?   @relation("LocationToLocation_parent", fields:[parentId], references: [id], onDelete: NoAction, onUpdate: NoAction)
            parentId  String?     @map("parent")
            children  Location[]  @relation("LocationToLocation_parent")
        }
    "#;

    api.schema_push_w_datasource(dm).send().assert_green();

    if api.is_mysql() {
        // MySQL creates an index for the FK.
        api.assert_schema().assert_table("Location", |t| {
            t.assert_indexes_count(1)
                .assert_index_on_columns(&["parent"], |idx| idx.assert_is_not_unique())
        });
    } else {
        api.assert_schema()
            .assert_table("Location", |t| t.assert_indexes_count(0));
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

    api.schema_push_w_datasource(dm).send().assert_green();
    api.assert_schema()
        .assert_table("Like", |table| table.assert_indexes_count(3));
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

    api.schema_push_w_datasource(dm1).send().assert_green();

    api.assert_schema().assert_table("A", |table| {
        table.assert_index_on_columns(&["field", "secondField"], |idx| idx.assert_is_unique())
    });

    let dm2 = r#"
        model A {
            id    Int    @id
            field String
            secondField Int
        }
    "#;

    api.schema_push_w_datasource(dm2).send().assert_green();

    api.assert_schema()
        .assert_table("A", |table| table.assert_indexes_count(0));
}

#[test_connector]
fn index_renaming_must_work(api: TestApi) {
    let dm1 = r#"
        model A {
            id Int @id
            field String
            secondField Int

            @@unique([field, secondField], map: "customName")
            @@index([secondField, field], name: "customNameNonUnique")
        }
    "#;

    api.schema_push_w_datasource(dm1).send().assert_green();

    api.assert_schema().assert_table("A", |table| {
        table
            .assert_index_on_columns(&["field", "secondField"], |idx| {
                idx.assert_is_unique().assert_name("customName")
            })
            .assert_index_on_columns(&["secondField", "field"], |idx| {
                idx.assert_is_not_unique().assert_name("customNameNonUnique")
            })
    });

    let dm2 = r#"
        model A {
            id Int @id
            field String
            secondField Int

            @@unique([field, secondField], map: "customNameA")
            @@index([secondField, field], map: "customNameNonUniqueA")
        }
    "#;

    api.schema_push_w_datasource(dm2).send().assert_green();

    api.assert_schema().assert_table("A", |table| {
        table
            .assert_indexes_count(2)
            .assert_index_on_columns(&["field", "secondField"], |idx| {
                idx.assert_is_unique().assert_name("customNameA")
            })
            .assert_index_on_columns(&["secondField", "field"], |idx| {
                idx.assert_is_not_unique().assert_name("customNameNonUniqueA")
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

            @@unique([field, secondField], map: "customName")
        }
    "#;

    api.schema_push_w_datasource(dm1).send().assert_green();
    api.assert_schema().assert_table("A", |t| {
        t.assert_index_on_columns(&["field", "secondField"], |idx| {
            idx.assert_is_unique().assert_name("customName")
        })
    });

    let dm2 = r#"
        model A {
            id Int @id
            field String
            secondField Int

            @@unique([field, secondField])
        }
    "#;

    api.schema_push_w_datasource(dm2).send();
    api.assert_schema().assert_table("A", |t| {
        t.assert_index_on_columns(&["field", "secondField"], |idx| {
            idx.assert_is_unique().assert_name("A_field_secondField_key")
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

    api.schema_push_w_datasource(dm1).send().assert_green();

    api.assert_schema().assert_table("A", |table| {
        table
            .assert_indexes_count(1)
            .assert_index_on_columns(&["field", "secondField"], |idx| idx.assert_is_unique())
    });

    let dm2 = r#"
        model A {
            id Int @id
            field String
            secondField Int

            @@unique([field, secondField], map: "somethingCustom")
        }
    "#;

    api.schema_push_w_datasource(dm2).send().assert_green();

    api.assert_schema().assert_table("A", |table| {
        table
            .assert_indexes_count(1)
            .assert_index_on_columns(&["field", "secondField"], |idx| {
                idx.assert_name("somethingCustom").assert_is_unique()
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

    api.schema_push_w_datasource(dm1).send().assert_green();
    api.assert_schema().assert_table("A", |t| {
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

    api.schema_push_w_datasource(dm2).force(true).send().assert_executable();

    api.assert_schema().assert_table("A", |t| {
        t.assert_indexes_count(1)
            .assert_index_on_columns(&["field", "id"], |idx| idx)
    });
}

#[test_connector]
fn dropping_a_model_with_a_multi_field_unique_index_must_work(api: TestApi) {
    let dm1 = r#"
        model A {
            id Int @id
            field String
            secondField Int

            @@unique([field, secondField], map: "customName")
        }
    "#;

    api.schema_push_w_datasource(dm1).send().assert_green();
    api.assert_schema().assert_table("A", |t| {
        t.assert_index_on_columns(&["field", "secondField"], |idx| {
            idx.assert_name("customName").assert_is_unique()
        })
    });

    api.schema_push_w_datasource("").send().assert_green();
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

    api.schema_push_w_datasource(dm).send().assert_green();

    api.assert_schema().assert_table("TestModelWithALongName", |table| {
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
                    "TestModelWithALongName_looooooooooooongfield_evenLongerField_idx"
                } else {
                    "TestModelWithALongName_looooooooooooongfield_evenLongerFiel_idx"
                })
            },
        )
    });

    api.schema_push_w_datasource(dm).send().assert_green().assert_no_steps();
}

#[test_connector]
fn new_index_with_same_name_as_index_from_dropped_table_works(api: TestApi) {
    let dm1 = r#"
        model Cat {
            id Int @id
            ownerid Int
            owner Owner @relation(fields: [ownerid], references: id)

            @@index([ownerid])
        }

        model Other {
            id Int @id
            ownerid Int

            @@index([ownerid], name: "ownerid")
        }

        model Owner {
            id Int @id
            c  Cat[]
        }
    "#;

    api.schema_push_w_datasource(dm1).send().assert_green();

    api.assert_schema().assert_table("Cat", |table| {
        table.assert_column("ownerid", |col| col.assert_is_required())
    });

    let dm2 = r#"
        model Owner {
            id      Int @id
            ownerid Int
            owner   Cat @relation(fields: [ownerid], references: id)

            @@index([ownerid], name: "ownerid")
        }

        model Cat {
            id      Int @id
            owners  Owner[]
        }
    "#;

    api.schema_push_w_datasource(dm2).send().assert_green();

    api.assert_schema().assert_table("Owner", |table| {
        table.assert_column("ownerid", |col| col.assert_is_required())
    });
}

#[test_connector]
fn column_type_migrations_should_not_implicitly_drop_indexes(api: TestApi) {
    let dm1 = r#"
        model Cat {
            id Int @id
            name String

            @@index([name])
        }
    "#;

    api.schema_push_w_datasource(dm1).send().assert_green();

    let dm2 = r#"
        model Cat {
            id Int @id
            name Int

            @@index([name])
        }
    "#;

    // NOTE: we are relying on the fact that we will drop and recreate the column for that particular type migration.
    api.schema_push_w_datasource(dm2).send().assert_green();
    api.assert_schema().assert_table("Cat", |cat| {
        cat.assert_indexes_count(1)
            .assert_index_on_columns(&["name"], |idx| idx.assert_is_not_unique())
    });
}

#[test_connector]
fn column_type_migrations_should_not_implicitly_drop_compound_indexes(api: TestApi) {
    let dm1 = r#"
        model Cat {
            id Int @id
            name String
            age Int

            @@index([name, age])
        }
    "#;

    api.schema_push_w_datasource(dm1).send().assert_green();

    let dm2 = r#"
        model Cat {
            id Int @id
            name Int
            age Int

            @@index([name, age])
        }
    "#;

    // NOTE: we are relying on the fact that we will drop and recreate the column for that particular type migration.
    api.schema_push_w_datasource(dm2).send().assert_green();

    api.assert_schema().assert_table("Cat", |cat| {
        cat.assert_indexes_count(1)
            .assert_index_on_columns(&["name", "age"], |idx| idx.assert_is_not_unique())
    });
}

#[test_connector(tags(Mysql8))]
fn length_prefixed_index(api: TestApi) {
    let dm = formatdoc! {r#"
        {}

        model A {{
          id Int    @id
          a  String @db.Text
          b  String @db.Text

          @@index([a(length: 30), b(length: 20)])
        }}
    "#, api.datasource_block()};

    api.schema_push(&dm).send().assert_green();

    api.assert_schema().assert_table("A", |table| {
        table.assert_index_on_columns(&["a", "b"], |index| {
            index
                .assert_is_not_unique()
                .assert_column("a", |attrs| attrs.assert_length_prefix(30))
                .assert_column("b", |attrs| attrs.assert_length_prefix(20))
        })
    });
}

#[test_connector(tags(Mysql8))]
fn length_prefixed_unique(api: TestApi) {
    let dm = formatdoc! {r#"
        {}

        model A {{
          id Int    @id
          a  String @db.Text
          b  String @db.Text

          @@unique([a(length: 30), b(length: 20)])
        }}
    "#, api.datasource_block()};

    api.schema_push(&dm).send().assert_green();

    api.assert_schema().assert_table("A", |table| {
        table.assert_index_on_columns(&["a", "b"], |index| {
            index
                .assert_is_unique()
                .assert_column("a", |attrs| attrs.assert_length_prefix(30))
                .assert_column("b", |attrs| attrs.assert_length_prefix(20))
        })
    });
}

#[test_connector(tags(Mysql8))]
fn removal_length_prefix_unique(api: TestApi) {
    let dm = formatdoc! {r#"
        {}

        model A {{
          id Int    @id
          a  String @db.VarChar(255)
          b  String @db.VarChar(255)

          @@unique([a(length: 30), b(length: 20)])
        }}
    "#, api.datasource_block()};

    api.schema_push(&dm).send().assert_green();

    api.assert_schema().assert_table("A", |table| {
        table.assert_index_on_columns(&["a", "b"], |index| {
            index
                .assert_is_unique()
                .assert_column("a", |attrs| attrs.assert_length_prefix(30))
                .assert_column("b", |attrs| attrs.assert_length_prefix(20))
        })
    });

    let dm = formatdoc! {r#"
        {}

        model A {{
          id Int    @id
          a  String @db.VarChar(255)
          b  String @db.VarChar(255)

          @@unique([a, b])
        }}
    "#, api.datasource_block()};

    api.schema_push(&dm).force(true).send().assert_warnings(&[
        "A unique constraint covering the columns `[a,b]` on the table `A` will be added. If there are existing duplicate values, this will fail.".into()
    ]);

    api.assert_schema().assert_table("A", |table| {
        table.assert_index_on_columns(&["a", "b"], |index| {
            index
                .assert_is_unique()
                .assert_column("a", |attrs| attrs.assert_no_length_prefix())
                .assert_column("b", |attrs| attrs.assert_no_length_prefix())
        })
    });
}

#[test_connector(tags(Mysql8))]
fn removal_length_prefix_index(api: TestApi) {
    let dm = formatdoc! {r#"
        {}

        model A {{
          id Int    @id
          a  String @db.VarChar(255)
          b  String @db.VarChar(255)

          @@index([a(length: 30), b(length: 20)])
        }}
    "#, api.datasource_block()};

    api.schema_push(&dm).send().assert_green();

    api.assert_schema().assert_table("A", |table| {
        table.assert_index_on_columns(&["a", "b"], |index| {
            index
                .assert_is_not_unique()
                .assert_column("a", |attrs| attrs.assert_length_prefix(30))
                .assert_column("b", |attrs| attrs.assert_length_prefix(20))
        })
    });

    let dm = formatdoc! {r#"
        {}

        model A {{
          id Int    @id
          a  String @db.VarChar(255)
          b  String @db.VarChar(255)

          @@index([a, b])
        }}
    "#, api.datasource_block()};

    api.schema_push(&dm).send().assert_green();

    api.assert_schema().assert_table("A", |table| {
        table.assert_index_on_columns(&["a", "b"], |index| {
            index
                .assert_is_not_unique()
                .assert_column("a", |attrs| attrs.assert_no_length_prefix())
                .assert_column("b", |attrs| attrs.assert_no_length_prefix())
        })
    });
}

#[test_connector(exclude(Mysql56, Mysql57, Mariadb))]
fn descending_compound_index(api: TestApi) {
    let dm = formatdoc! {r#"
        {}

        model A {{
          id Int    @id
          a  Int
          b  Int

          @@index([a, b(sort: Desc)])
        }}
    "#, api.datasource_block()};

    api.schema_push(&dm).send().assert_green();

    api.assert_schema().assert_table("A", |table| {
        table.assert_index_on_columns(&["a", "b"], |index| {
            index
                .assert_is_not_unique()
                .assert_column("a", |attrs| attrs.assert_sort_order(SQLSortOrder::Asc))
                .assert_column("b", |attrs| attrs.assert_sort_order(SQLSortOrder::Desc))
        })
    });
}

#[test_connector(exclude(Mysql56, Mysql57, Mariadb))]
fn descending_compound_unique(api: TestApi) {
    let dm = formatdoc! {r#"
        {}

        model A {{
          id Int    @id
          a  Int
          b  Int

          @@unique([a, b(sort: Desc)])
        }}
    "#, api.datasource_block()};

    api.schema_push(&dm).send().assert_green();

    api.assert_schema().assert_table("A", |table| {
        table.assert_index_on_columns(&["a", "b"], |index| {
            index
                .assert_is_unique()
                .assert_column("a", |attrs| attrs.assert_sort_order(SQLSortOrder::Asc))
                .assert_column("b", |attrs| attrs.assert_sort_order(SQLSortOrder::Desc))
        })
    });
}

#[test_connector(exclude(Mysql56, Mysql57, Mariadb))]
fn descending_unique(api: TestApi) {
    let dm = formatdoc! {r#"
        {}

        model A {{
          id Int @id
          a  Int @unique(sort: Desc)
        }}
    "#, api.datasource_block()};

    api.schema_push(&dm).send().assert_green();

    api.assert_schema().assert_table("A", |table| {
        table.assert_index_on_columns(&["a"], |index| {
            index
                .assert_is_unique()
                .assert_column("a", |attrs| attrs.assert_sort_order(SQLSortOrder::Desc))
        })
    });
}

#[test_connector(exclude(Mysql56, Mysql57, Mariadb))]
fn removal_descending_unique(api: TestApi) {
    let dm = formatdoc! {r#"
        {}

        model A {{
          id Int @id
          a  Int @unique(sort: Desc)
        }}
    "#, api.datasource_block()};

    api.schema_push(&dm).send().assert_green();

    api.assert_schema().assert_table("A", |table| {
        table.assert_index_on_columns(&["a"], |index| {
            index
                .assert_is_unique()
                .assert_column("a", |attrs| attrs.assert_sort_order(SQLSortOrder::Desc))
        })
    });

    let dm = formatdoc! {r#"
        {}

        model A {{
          id Int @id
          a  Int @unique
        }}
    "#, api.datasource_block()};

    api.schema_push(dm)
        .force(true)
        .send()
        .assert_warnings(&["A unique constraint covering the columns `[a]` on the table `A` will be added. If there are existing duplicate values, this will fail.".into()]);

    api.assert_schema().assert_table("A", |table| {
        table.assert_index_on_columns(&["a"], |index| {
            index
                .assert_is_unique()
                .assert_column("a", |attrs| attrs.assert_sort_order(SQLSortOrder::Asc))
        })
    });
}

#[test_connector(tags(Mysql8))]
fn removal_descending_unique_should_not_happen_without_preview_feature(api: TestApi) {
    let sql = indoc! {r#"
        CREATE TABLE `A` (
            id INT PRIMARY KEY,
            a INT NOT NULL,
            CONSTRAINT A_a_key UNIQUE (a DESC)
        )
    "#};

    api.raw_cmd(sql);

    let dm = indoc! {r#"
        model A {
          id Int @id
          a  Int @unique
        }
    "#};

    api.schema_push_w_datasource(dm).send().assert_no_steps();
}

#[test_connector(tags(Mysql8))]
fn removal_index_length_prefix_should_not_happen_without_preview_feature(api: TestApi) {
    let sql = indoc! {r#"
        CREATE TABLE `A` (
            id INT PRIMARY KEY,
            a VARCHAR(255) NOT NULL,
            CONSTRAINT A_a_key UNIQUE (a(30))
        )
    "#};

    api.raw_cmd(sql);

    let dm = indoc! {r#"
        model A {
          id Int    @id
          a  String @unique @db.VarChar(255)
        }
    "#};

    api.schema_push_w_datasource(dm).send().assert_no_steps();
}

#[test_connector(tags(Mysql))]
fn fulltext_index(api: TestApi) {
    let dm = formatdoc! {r#"
        {}

        generator client {{
          provider = "prisma-client-js"
        }}

        model A {{
          id Int    @id
          a  String @db.Text
          b  String @db.Text

          @@fulltext([a, b])
        }}
    "#, api.datasource_block()};

    api.schema_push(&dm).send().assert_green();

    api.assert_schema().assert_table("A", |table| {
        table.assert_index_on_columns(&["a", "b"], |index| index.assert_is_fulltext().assert_name("A_a_b_idx"))
    });
}

#[test_connector(tags(Mysql))]
fn fulltext_index_with_map(api: TestApi) {
    let dm = indoc! {r#"
        datasource db {
            provider = "mysql"
            url = env("TEST_DATABASE_URL")
        }

        generator client {
          provider = "prisma-client-js"
        }

        model A {
          id Int    @id
          a  String @db.Text
          b  String @db.Text

          @@fulltext([a, b], map: "with_map")
        }
    "#};

    api.schema_push(dm).send().assert_green();

    api.assert_schema().assert_table("A", |table| {
        table.assert_index_on_columns(&["a", "b"], |index| index.assert_is_fulltext().assert_name("with_map"))
    });
}

#[test_connector(tags(Mysql))]
fn adding_fulltext_index_to_an_existing_column(api: TestApi) {
    let dm = indoc! {r#"
        model A {
          id Int    @id
          a  String @db.Text
          b  String @db.Text
        }
    "#};

    api.schema_push(api.datamodel_with_provider(dm)).send().assert_green();

    api.assert_schema()
        .assert_table("A", |table| table.assert_indexes_count(0));

    let dm = indoc! {r#"
        model A {
          id Int    @id
          a  String @db.Text
          b  String @db.Text

          @@fulltext([a, b])
        }
    "#};

    api.schema_push(api.datamodel_with_provider(dm)).send().assert_green();

    api.assert_schema().assert_table("A", |table| {
        table.assert_index_on_columns(&["a", "b"], |index| index.assert_is_fulltext())
    });
}

#[test_connector(tags(Mysql), exclude(Mysql56))]
fn changing_normal_index_to_a_fulltext_index(api: TestApi) {
    let dm = indoc! {r#"
        model A {
          id Int    @id
          a  String @db.VarChar(255)
          b  String @db.VarChar(255)

          @@index([a, b])
        }
    "#};

    api.schema_push(api.datamodel_with_provider(dm)).send().assert_green();

    api.assert_schema().assert_table("A", |table| {
        table.assert_indexes_count(1);
        table.assert_index_on_columns(&["a", "b"], |index| index.assert_is_normal())
    });

    let dm = indoc! {r#"
        model A {
          id Int    @id
          a  String @db.VarChar(255)
          b  String @db.VarChar(255)

          @@fulltext([a, b])
        }
    "#};

    api.schema_push(api.datamodel_with_provider(dm)).send().assert_green();

    api.assert_schema().assert_table("A", |table| {
        table.assert_indexes_count(1);
        table.assert_index_on_columns(&["a", "b"], |index| index.assert_is_fulltext())
    });
}

#[test_connector]
fn changing_unique_to_pk_works(api: TestApi) {
    let dm1 = indoc! {r#"
        model A {
            id   Int     @unique
            name String?
        }

        model B {
            x Int
            y Int

            @@unique([x, y])
        }
    "#};

    api.schema_push_w_datasource(dm1).send().assert_green();

    api.assert_schema()
        .assert_table("A", |table| {
            table.assert_index_on_columns(&["id"], |index| index.assert_is_unique())
        })
        .assert_table("B", |table| {
            table.assert_index_on_columns(&["x", "y"], |index| index.assert_is_unique())
        });

    let dm2 = indoc! {r#"
        model A {
            id Int @id
        }

        model B {
            x Int
            y Int

            @@id([x, y])
        }
    "#};

    api.schema_push_w_datasource(dm2).send().assert_green();

    api.assert_schema()
        .assert_table("A", |table| {
            table.assert_pk(|pk| pk.assert_columns(&["id"]))
        })
        .assert_table("B", |table| {
            table.assert_pk(|pk| pk.assert_columns(&["x", "y"]))
        });
}
