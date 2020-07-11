use crate::*;
use barrel::types;
use pretty_assertions::assert_eq;
use test_harness::*;

#[test_each_connector(tags("sqlite"))]
#[test]
async fn introspecting_a_simple_table_with_gql_types_must_work(api: &TestApi) {
    let barrel = api.barrel();
    let _setup_schema = barrel
        .execute(|migration| {
            migration.create_table("Blog", |t| {
                t.add_column("bool", types::boolean());
                t.add_column("float", types::float());
                t.add_column("date", types::date());
                t.add_column("id", types::primary());
                t.add_column("integer", types::integer());
                t.inject_custom("int int not null");
                t.add_column("string", types::text());
            });
        })
        .await;
    let dm = r#"
            model Blog {
                bool    Boolean
                float   Float
                date    DateTime
                id      Int @id @default(autoincrement())
                integer Int
                int     Int
                string  String
            }
        "#;
    let result = dbg!(api.introspect().await);
    custom_assert(&result, dm);
}

#[test_each_connector(tags("sqlite"))]
async fn introspecting_a_table_with_compound_primary_keys_must_work(api: &TestApi) {
    let barrel = api.barrel();
    let _setup_schema = barrel
        .execute(|migration| {
            migration.create_table("Blog", |t| {
                t.add_column("id", types::integer());
                t.add_column("authorId", types::text());
                t.inject_custom("PRIMARY KEY (\"id\", \"authorId\")");
            });
        })
        .await;

    let dm = r#"
            model Blog {
                id Int
                authorId String
                @@id([id, authorId])
            }
        "#;
    let result = dbg!(api.introspect().await);
    custom_assert(&result, dm);
}

#[test_each_connector(tags("sqlite"))]
async fn introspecting_a_table_with_unique_index_must_work(api: &TestApi) {
    let barrel = api.barrel();
    let _setup_schema = barrel
        .execute(|migration| {
            migration.create_table("Blog", |t| {
                t.add_column("id", types::primary());
                t.add_column("authorId", types::text());
                t.add_index("test", types::index(vec!["authorId"]).unique(true));
            });
        })
        .await;

    let dm = r#"
            model Blog {
                id Int @id @default(autoincrement())
                authorId String @unique
            }
        "#;
    let result = dbg!(api.introspect().await);
    custom_assert(&result, dm);
}

#[test_each_connector(tags("sqlite"))]
async fn introspecting_a_table_with_multi_column_unique_index_must_work(api: &TestApi) {
    let barrel = api.barrel();
    barrel
        .execute(|migration| {
            migration.create_table("User", |t| {
                t.add_column("id", types::primary());
                t.add_column("firstname", types::text());
                t.add_column("lastname", types::text());
                t.add_index("test", types::index(vec!["firstname", "lastname"]).unique(true));
            });
        })
        .await;

    let dm = r#"
            model User {
                id Int @id @default(autoincrement())
                firstname String
                lastname String
                @@unique([firstname, lastname], name: "test")
            }
        "#;
    let result = dbg!(api.introspect().await);
    custom_assert(&result, dm);
}

#[test_each_connector(tags("sqlite"))]
async fn introspecting_a_table_with_required_and_optional_columns_must_work(api: &TestApi) {
    let barrel = api.barrel();
    barrel
        .execute(|migration| {
            migration.create_table("User", |t| {
                t.add_column("id", types::primary());
                t.add_column("requiredname", types::text().nullable(false));
                t.add_column("optionalname", types::text().nullable(true));
            });
        })
        .await;

    let dm = r#"
            model User {
                id Int @id @default(autoincrement())
                requiredname String
                optionalname String?
            }
        "#;
    let result = dbg!(api.introspect().await);
    custom_assert(&result, dm);
}

//#[test_each_connector(tags("sqlite"))]
//#[ignore]
//fn introspecting_a_table_with_datetime_default_values_should_work(api: &TestApi) {
//    let barrel = api.barrel();
//    let _setup_schema = barrel.execute(|migration| {
//        migration.create_table("User", |t| {
//            t.add_column("id", types::primary());
//            t.add_column("name", types::text());
//            t.inject_custom("\"joined\" date DEFAULT CURRENT_DATE")
//        });
//    });
//
//    let dm = r#"
//            model User {
//                id Int @id
//                joined DateTime? @default(now())
//                name String
//            }
//        "#;
//    let result = dbg!(api.introspect().await);
//    custom_assert(&result, dm);
//}

#[test_each_connector(tags("sqlite"))]
async fn introspecting_a_table_with_default_values_should_work(api: &TestApi) {
    let barrel = api.barrel();
    let _setup_schema = barrel
        .execute(|migration| {
            migration.create_table("User", |t| {
                t.add_column("a", types::text());
                t.add_column("id", types::primary());
                t.inject_custom("\"bool\" Boolean NOT NULL DEFAULT false");
                t.inject_custom("\"bool2\" Boolean NOT NULL DEFAULT 0");
                t.inject_custom("\"float\" Float NOT NULL DEFAULT 5.3");
                t.inject_custom("\"int\" INTEGER NOT NULL DEFAULT 5");
                t.inject_custom("\"string\" TEXT NOT NULL DEFAULT \"Test\"");
            });
        })
        .await;

    let dm = r#"
            model User {
                a String
                id Int @id @default(autoincrement())
                bool Boolean @default(false)
                bool2 Boolean @default(false)
                float Float @default(5.3)
                int Int @default(5)
                string String @default("Test")
            }
        "#;
    let result = dbg!(api.introspect().await);
    custom_assert(&result, dm);
}

#[test_each_connector(tags("sqlite"))]
async fn introspecting_a_table_with_a_non_unique_index_should_work(api: &TestApi) {
    let barrel = api.barrel();
    let _setup_schema = barrel
        .execute(|migration| {
            migration.create_table("User", |t| {
                t.add_column("a", types::text());
                t.add_column("id", types::primary());
                t.add_index("test", types::index(vec!["a"]));
            });
        })
        .await;

    let dm = r#"
            model User {
                a String
                id Int @id @default(autoincrement())
                @@index([a], name: "test")
            }
        "#;
    let result = dbg!(api.introspect().await);
    custom_assert(&result, dm);
}

#[test_each_connector(tags("sqlite"))]
async fn introspecting_a_table_with_a_multi_column_non_unique_index_should_work(api: &TestApi) {
    let barrel = api.barrel();
    let _setup_schema = barrel
        .execute(|migration| {
            migration.create_table("User", |t| {
                t.add_column("a", types::text());
                t.add_column("b", types::text());
                t.add_column("id", types::primary());
                t.add_index("test", types::index(vec!["a", "b"]));
            });
        })
        .await;

    let dm = r#"
            model User {
                a String
                b String
                id Int @id @default(autoincrement())
                @@index([a,b], name: "test")
            }
        "#;
    let result = dbg!(api.introspect().await);
    custom_assert(&result, dm);
}

#[test_each_connector(tags("sqlite"))]
async fn introspecting_a_table_with_optional_autoincrement_should_work(api: &TestApi) {
    api.barrel()
        .execute(|migration| {
            migration.create_table("Book", |t| {
                t.inject_custom("book_id Integer Primary Key Autoincrement");
            });
        })
        .await;

    let dm = r#"
        model Book {
            book_id      Int     @default(autoincrement()) @id
        }
    "#;

    let result = dbg!(api.introspect().await);
    custom_assert(&result, dm);
}

#[test_each_connector(tags("sqlite"))]
async fn introspecting_a_table_without_uniques_should_comment_it_out(api: &TestApi) {
    api.barrel()
        .execute(|migration| {
            migration.create_table("User", |t| {
                t.add_column("id", types::primary());
            });
            migration.create_table("Post", |t| {
                t.add_column("id", types::integer());
                t.inject_custom(
                    "user_id INTEGER NOT NULL,
                FOREIGN KEY (`user_id`) REFERENCES `User`(`id`)",
                )
            });
        })
        .await;

    let dm = "model User {\n  id      Int    @default(autoincrement()) @id\n  // Post Post[]\n}\n\n// The underlying table does not contain a unique identifier and can therefore currently not be handled.\n// model Post {\n  // id      Int\n  // user_id Int\n  // User    User @relation(fields: [user_id], references: [id])\n// }\n";

    let result = dbg!(api.introspect().await);
    assert_eq!(&result, dm);
}

#[test_each_connector(tags("sqlite"))]
async fn introspecting_a_default_value_as_dbgenerated_should_work(api: &TestApi) {
    let barrel = api.barrel();
    let _setup_schema = barrel
        .execute(|migration| {
            migration.create_table("Test", |t| {
                t.add_column("id", types::primary());
                t.inject_custom("string_static_char varchar(5) Default 'test'");
                t.inject_custom("string_static_null varchar(5) Default NULL");
                t.inject_custom("string_static_text text Default 'test'");
                t.inject_custom("string_static_varchar varchar(5) Default 'test'");
                t.inject_custom("int_static Integer DEFAULT 2");
                t.inject_custom("float_static Float DEFAULT 1.43");
                t.inject_custom("boolean_static_1 Boolean DEFAULT 1");
                t.inject_custom("boolean_static_true Boolean DEFAULT tRue");
                t.inject_custom("datetime_now_current_timestamp TIMESTAMP NULL DEFAULT CURRENT_TIMESTAMP");
                t.inject_custom("datetime_now_current_timestamp_lc TIMESTAMP NULL DEFAULT current_TIMESTAMP");
                t.inject_custom("datetime_now_datetime_now TIMESTAMP NULL DEFAULT (DATETIME('now'))");
                t.inject_custom(
                    "datetime_datetime_now_localtime TIMESTAMP NULL DEFAULT (datetime('now', 'localtime'))",
                );
            });
        })
        .await;

    let dm = r#"
            model Test {
                id                                  Int         @default(autoincrement()) @id
                string_static_char                  String?     @default("test")
                string_static_null                  String?
                string_static_text                  String?     @default("test")
                string_static_varchar               String?     @default("test")
                int_static                          Int?        @default(2)
                float_static                        Float?      @default(1.43)
                boolean_static_1                    Boolean?    @default(true)
                boolean_static_true                 Boolean?    @default(true)
                datetime_now_current_timestamp      DateTime?   @default(now())
                datetime_now_current_timestamp_lc   DateTime?   @default(now())
                datetime_now_datetime_now           DateTime?   @default(now())
                datetime_datetime_now_localtime     DateTime?   @default(now())
            }
        "#;

    let result = dbg!(api.introspect().await);
    custom_assert(&result, dm);
}
