use crate::*;
use barrel::types;
use test_harness::*;

#[test_one_connector(connector = "mysql")]
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
                date    DateTime
                float   Float
                id      Int @id @default(autoincrement())
                int     Int
                string  String
            }
        "#;
    let result = dbg!(api.introspect().await);
    custom_assert(&result, dm);
}

#[test_one_connector(connector = "mysql")]
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
                authorId String
                id Int
                @@id([id, authorId])
            }
        "#;
    let result = dbg!(api.introspect().await);
    custom_assert(&result, dm);
}

#[test_one_connector(connector = "mysql")]
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
                authorId String @unique
                id      Int @id @default(autoincrement())
            }
        "#;
    let result = dbg!(api.introspect().await);
    custom_assert(&result, dm);
}

#[test_one_connector(connector = "mysql")]
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
                firstname String
                id      Int @id @default(autoincrement())
                lastname String
                @@unique([firstname, lastname], name: "test")
            }
        "#;
    let result = dbg!(api.introspect().await);
    custom_assert(&result, dm);
}

#[test_one_connector(connector = "mysql")]
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
                optionalname String?
                requiredname String
            }
        "#;
    let result = dbg!(api.introspect().await);
    custom_assert(&result, dm);
}

//#[test_one_connector(connector = "mysql")]
//#[ignore]
//fn introspecting_a_table_with_datetime_default_values_should_work(api: &TestApi) {
//    let barrel = api.barrel();
//    let _setup_schema = barrel.execute_with_schema(, api.db_name(|migration| ){
//        migration.create_table("User", |t| {
//            t.add_column("id", types::primary());
//            t.add_column("name", types::text());
//            t.inject_custom("`joined` date DEFAULT CURRENT_DATE")
//        });
//    }).await;
//
//    let dm = r#"
//            model User {
//                id      Int @id
//                joined DateTime? @default(now())
//                name String
//            }
//        "#;
//    let result = dbg!(api.introspect().await);
//    custom_assert(&result, dm);
//}

#[test_one_connector(connector = "mysql")]
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
                bool Boolean @default(false)
                bool2 Boolean @default(false)
                float Float @default(5.3)
                id      Int @id @default(autoincrement())
                int Int @default(5)
                string String @default("Test")
            }
        "#;
    let result = dbg!(api.introspect().await);
    custom_assert(&result, dm);
}

#[test_one_connector(connector = "mysql")]
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

#[test_one_connector(connector = "mysql")]
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
