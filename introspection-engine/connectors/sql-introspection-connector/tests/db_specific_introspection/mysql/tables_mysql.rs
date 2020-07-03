use crate::*;
use barrel::types;
use pretty_assertions::assert_eq;
use test_harness::*;

#[test_each_connector(tags("mysql"))]
async fn introspecting_a_simple_table_with_gql_types_must_work(api: &TestApi) {
    let barrel = api.barrel();
    let _setup_schema = barrel
        .execute_with_schema(
            |migration| {
                migration.create_table("Blog", |t| {
                    t.add_column("bool", types::boolean());
                    t.add_column("float", types::float());
                    t.add_column("date", types::date());
                    t.add_column("id", types::primary());
                    t.add_column("int", types::integer());
                    t.add_column("string", types::text());
                });
            },
            api.db_name(),
        )
        .await;
    let dm = r#"    
            model Blog {
                bool    Boolean
                float   Float
                date    DateTime
                id      Int @id @default(autoincrement())
                int     Int
                string  String
            }
        "#;
    let result = dbg!(api.introspect().await);
    custom_assert(&result, dm);
}

#[test_each_connector(tags("mysql_8"))]
async fn introspecting_a_table_with_json_type_must_work(api: &TestApi) {
    let barrel = api.barrel();
    let _setup_schema = barrel
        .execute_with_schema(
            |migration| {
                migration.create_table("Blog", |t| {
                    t.add_column("id", types::primary());
                    t.add_column("json", types::json());
                });
            },
            api.db_name(),
        )
        .await;
    let dm = r#"
            datasource mysql {
                provider = "mysql"
                url = "mysql://asdlj"
            }
    
            model Blog {
                id      Int @id @default(autoincrement())
                json    Json
            }
        "#;
    let result = dbg!(api.introspect().await);
    custom_assert(&result, dm);
}

#[test_each_connector(tags("mysql"))]
async fn introspecting_a_table_with_compound_primary_keys_must_work(api: &TestApi) {
    let barrel = api.barrel();
    let _setup_schema = barrel
        .execute_with_schema(
            |migration| {
                migration.create_table("Blog", |t| {
                    t.add_column("id", types::integer());
                    t.add_column("authorId", types::varchar(10));
                    t.inject_custom("PRIMARY KEY (`id`, `authorId`)");
                });
            },
            api.db_name(),
        )
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

#[test_each_connector(tags("mysql"))]
async fn introspecting_a_table_with_unique_index_must_work(api: &TestApi) {
    let barrel = api.barrel();
    let _setup_schema = barrel
        .execute_with_schema(
            |migration| {
                migration.create_table("Blog", |t| {
                    t.add_column("id", types::primary());
                    t.add_column("authorId", types::varchar(10));
                    t.add_index("test", types::index(vec!["authorId"]).unique(true));
                });
            },
            api.db_name(),
        )
        .await;

    let dm = r#"
            model Blog {
                id      Int @id @default(autoincrement())
                authorId String @unique
            }
        "#;
    let result = dbg!(api.introspect().await);
    custom_assert(&result, dm);
}

#[test_each_connector(tags("mysql"))]
async fn introspecting_a_table_with_multi_column_unique_index_must_work(api: &TestApi) {
    let barrel = api.barrel();
    barrel
        .execute_with_schema(
            |migration| {
                migration.create_table("User", |t| {
                    t.add_column("id", types::primary());
                    t.add_column("firstname", types::varchar(10));
                    t.add_column("lastname", types::varchar(10));
                    t.add_index("test", types::index(vec!["firstname", "lastname"]).unique(true));
                });
            },
            api.db_name(),
        )
        .await;

    let dm = r#"
            model User {
                id      Int @id @default(autoincrement())
                firstname String
                lastname String
                @@unique([firstname, lastname], name: "test")
            }
        "#;
    let result = dbg!(api.introspect().await);
    custom_assert(&result, dm);
}

#[test_each_connector(tags("mysql"))]
async fn introspecting_a_table_with_required_and_optional_columns_must_work(api: &TestApi) {
    let barrel = api.barrel();
    let _setup_schema = barrel
        .execute_with_schema(
            |migration| {
                migration.create_table("User", |t| {
                    t.add_column("id", types::primary());
                    t.add_column("requiredname", types::text().nullable(false));
                    t.add_column("optionalname", types::text().nullable(true));
                });
            },
            api.db_name(),
        )
        .await;

    let dm = r#"
            model User {
                id      Int @id @default(autoincrement())
                requiredname String
                optionalname String?
            }
        "#;
    let result = dbg!(api.introspect().await);
    custom_assert(&result, dm);
}

// #[test_each_connector(tags("mysql"))]
// async fn introspecting_a_table_with_datetime_default_values_should_work(api: &TestApi) {
//     let barrel = api.barrel();
//     let _setup_schema = barrel
//         .execute_with_schema(
//             |migration| {
//                 migration.create_table("User", |t| {
//                     t.add_column("id", types::primary());
//                     t.add_column("name", types::text());
//                     t.inject_custom("`joined` date DEFAULT CURRENT_DATE")
//                 });
//             },
//             api.db_name(),
//         )
//         .await;
//
//     let dm = r#"
//             model User {
//                 id      Int @id
//                 joined DateTime? @default(now())
//                 name String
//             }
//         "#;
//     let result = dbg!(api.introspect().await);
//     custom_assert(&result, dm);
// }

#[test_each_connector(tags("mysql"))]
async fn introspecting_a_table_with_default_values_should_work(api: &TestApi) {
    let barrel = api.barrel();
    let _setup_schema = barrel
        .execute_with_schema(
            |migration| {
                migration.create_table("User", |t| {
                    t.add_column("a", types::text());
                    t.add_column("id", types::primary());
                    t.inject_custom("`bool` Boolean NOT NULL DEFAULT false");
                    t.inject_custom("`bool2` Boolean NOT NULL DEFAULT 0");
                    t.inject_custom("`float` Float NOT NULL DEFAULT 5.3");
                    t.inject_custom("`int` INTEGER NOT NULL DEFAULT 5");
                    t.inject_custom("`string` VARCHAR(4) NOT NULL DEFAULT 'Test'");
                });
            },
            api.db_name(),
        )
        .await;

    let dm = r#"
            model User {
                a String
                id      Int @id @default(autoincrement())
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

#[test_each_connector(tags("mysql"))]
async fn introspecting_a_table_with_a_non_unique_index_should_work(api: &TestApi) {
    let barrel = api.barrel();
    let _setup_schema = barrel
        .execute_with_schema(
            |migration| {
                migration.create_table("User", |t| {
                    t.add_column("a", types::varchar(10));
                    t.add_column("id", types::primary());
                    t.add_index("test", types::index(vec!["a"]));
                });
            },
            api.db_name(),
        )
        .await;

    let dm = r#"
            model User {
                a String
                id      Int @id @default(autoincrement())
                @@index([a], name: "test")
            }
        "#;
    let result = dbg!(api.introspect().await);
    custom_assert(&result, dm);
}

#[test_each_connector(tags("mysql"))]
async fn introspecting_a_table_with_a_multi_column_non_unique_index_should_work(api: &TestApi) {
    let barrel = api.barrel();
    let _setup_schema = barrel
        .execute_with_schema(
            |migration| {
                migration.create_table("User", |t| {
                    t.add_column("a", types::varchar(10));
                    t.add_column("b", types::varchar(10));
                    t.add_column("id", types::primary());
                    t.add_index("test", types::index(vec!["a", "b"]));
                });
            },
            api.db_name(),
        )
        .await;

    let dm = r#"
            model User {
                a String
                b String
                id      Int @id @default(autoincrement())
                @@index([a,b], name: "test")
            }
        "#;
    let result = dbg!(api.introspect().await);
    custom_assert(&result, dm);
}

#[test_each_connector(tags("mysql"))]
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

    let dm = "// The underlying table does not contain a unique identifier and can therefore currently not be handled.\n// model Post {\n  // id      Int\n  // user_id Int\n  // User    User @relation(fields: [user_id], references: [id])\n\n  // @@index([user_id], name: \"user_id\")\n// }\n\nmodel User {\n  id Int @default(autoincrement()) @id\n}";

    let result = dbg!(api.introspect().await);
    assert_eq!(&result, dm);
}

//todo maybe need to split due to
// no function default values on mysql 5.7 and 8.0 -.-
// maria db allows this
#[test_each_connector(tags("mysql"))]
async fn introspecting_a_default_value_as_dbgenerated_should_work(api: &TestApi) {
    let barrel = api.barrel();
    let _setup_schema = barrel
        .execute(|migration| {
            migration.create_table("Test", |t| {
                t.add_column("id", types::primary());
                t.inject_custom("string_static_char char(5) Default 'test'");
                t.inject_custom("string_static_char_null char(5) Default NULL");
                t.inject_custom("string_static_varchar varchar(5) Default 'test'");
                // t.inject_custom("string_function char(200) Default CONCAT('id','string_static_text')");
                t.inject_custom("int_static Integer DEFAULT 2");
                // t.inject_custom("int_function Integer DEFAULT FIELD('Bb', 'Aa', 'Bb', 'Cc', 'Dd', 'Ff')");
                t.inject_custom("float_static Float DEFAULT 1.43");
                t.inject_custom("boolean_static Boolean DEFAULT 1");
                t.inject_custom("datetime_now TIMESTAMP NULL DEFAULT CURRENT_TIMESTAMP");
                t.inject_custom("datetime_now_lc TIMESTAMP NULL DEFAULT current_timestamp");
                t.inject_custom("enum_static  ENUM ( 'black', 'white') Default 'black'");
            });
        })
        .await;

    let dm = r#"
            model Test {
                id                      Int                 @default(autoincrement()) @id
                string_static_char      String?             @default("test")
                string_static_char_null String?     
                string_static_varchar   String?             @default("test") 
                int_static              Int?                @default(2)
                float_static            Float?              @default(1.43)                   
                boolean_static          Boolean?            @default(true)
                datetime_now            DateTime?           @default(now())
                datetime_now_lc         DateTime?           @default(now())
                enum_static             Test_enum_static?   @default(black)   
            }
            
            enum Test_enum_static{
                black
                white
            }
        "#;

    let result = dbg!(api.introspect().await);
    custom_assert(&result, dm);
}
