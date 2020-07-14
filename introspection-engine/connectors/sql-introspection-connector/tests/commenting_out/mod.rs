use crate::*;
use barrel::types;
use pretty_assertions::assert_eq;
use test_harness::*;

#[test_each_connector(tags("sqlite"))]
async fn introspecting_a_table_without_uniques_should_comment_it_out_sqlite(api: &TestApi) {
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

    let dm = "model User {\n  id      Int    @default(autoincrement()) @id\n  // Post Post[]\n}\n\n// The underlying table does not contain a valid unique identifier and can therefore currently not be handled.\n// model Post {\n  // id      Int\n  // user_id Int\n  // User    User @relation(fields: [user_id], references: [id])\n// }\n";

    let result = dbg!(api.introspect().await);
    assert_eq!(&result, dm);
}

#[test_each_connector(tags("sqlite"))]
async fn introspecting_a_table_without_required_uniques_should_comment_it_out_sqlite(api: &TestApi) {
    api.barrel()
        .execute(|migration| {
            migration.create_table("Post", |t| {
                t.add_column("id", types::integer());
                t.add_column("opt_unique", types::integer().unique(true).nullable(true));
            });
        })
        .await;

    let dm = "// The underlying table does not contain a valid unique identifier and can therefore currently not be handled.\n// model Post {\n  // id         Int\n  // opt_unique Int? @unique\n// }\n";

    let result = dbg!(api.introspect().await);
    assert_eq!(&result, dm);
}

#[test_each_connector(tags("sqlite"))]
async fn introspecting_a_table_without_fully_required_compound_unique_should_comment_it_out_sqlite(api: &TestApi) {
    api.barrel()
        .execute(|migration| {
            migration.create_table("Post", |t| {
                t.add_column("id", types::integer());
                t.add_column("opt_unique", types::integer().nullable(true));
                t.add_column("req_unique", types::integer().nullable(false));
                t.inject_custom("Unique(opt_unique, req_unique)")
            });
        })
        .await;

    let dm = "// The underlying table does not contain a valid unique identifier and can therefore currently not be handled.\n// model Post {\n  // id         Int\n  // opt_unique Int?\n  // req_unique Int\n\n  // @@unique([opt_unique, req_unique], name: \"sqlite_autoindex_Post_1\")\n// }\n";

    let result = dbg!(api.introspect().await);
    assert_eq!(&result, dm);
}

#[test_each_connector(tags("mysql"))]
async fn introspecting_a_table_without_uniques_should_comment_it_out_mysql(api: &TestApi) {
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

    let dm = "// The underlying table does not contain a valid unique identifier and can therefore currently not be handled.\n// model Post {\n  // id      Int\n  // user_id Int\n  // User    User @relation(fields: [user_id], references: [id])\n\n  // @@index([user_id], name: \"user_id\")\n// }\n\nmodel User {\n  id      Int    @default(autoincrement()) @id\n  // Post Post[]\n}\n";

    let result = dbg!(api.introspect().await);
    assert_eq!(&result, dm);
}

#[test_each_connector(tags("postgres"))]
async fn introspecting_an_enum_with_an_invalid_value_should_work(api: &TestApi) {
    let sql = format!("CREATE Type status as ENUM ( '1', 'UNDEFINED')");

    api.database().execute_raw(&sql, &[]).await.unwrap();

    api.barrel()
        .execute(|migration| {
            migration.create_table("News", |t| {
                t.add_column("id", types::primary());
                t.inject_custom("status  status Not Null default 'UNDEFINED'");
            });
        })
        .await;

    let warnings = dbg!(api.introspection_warnings().await);
    assert_eq!(&warnings, "[{\"code\":4,\"message\":\"These enum values were commented out because their names are currently not supported by Prisma. Please provide valid ones that match [a-zA-Z][a-zA-Z0-9_]* using the `@map` directive.\",\"affected\":[{\"enm\":\"status\",\"value\":\"1\"}]}]");

    let result = dbg!(api.introspect().await);
    assert_eq!(&result, "model News {\n  id     Int    @default(autoincrement()) @id\n  status status @default(UNDEFINED)\n}\n\nenum status {\n  // 1 @map(\"1\")\n  UNDEFINED\n}\n");
}

#[test_each_connector(tags("postgres"))]
async fn introspecting_an_enum_with_an_invalid_value_as_default_should_work(api: &TestApi) {
    let sql = format!("CREATE Type status as ENUM ( '1', 'UNDEFINED')");

    api.database().execute_raw(&sql, &[]).await.unwrap();

    api.barrel()
        .execute(|migration| {
            migration.create_table("News", |t| {
                t.add_column("id", types::primary());
                t.inject_custom("status  status Not Null default '1'");
            });
        })
        .await;

    let warnings = dbg!(api.introspection_warnings().await);
    assert_eq!(&warnings, "[{\"code\":4,\"message\":\"These enum values were commented out because their names are currently not supported by Prisma. Please provide valid ones that match [a-zA-Z][a-zA-Z0-9_]* using the `@map` directive.\",\"affected\":[{\"enm\":\"status\",\"value\":\"1\"}]}]");

    let result = dbg!(api.introspect().await);
    assert_eq!(&result, "model News {\n  id     Int    @default(autoincrement()) @id\n  status status @default(dbgenerated())\n}\n\nenum status {\n  // 1 @map(\"1\")\n  UNDEFINED\n}\n");
}

#[test_each_connector(tags("postgres"))]
async fn introspecting_an_unsupported_type_should_and_commenting_it_out_should_also_drop_its_usages(api: &TestApi) {
    let barrel = api.barrel();
    let _setup_schema = barrel
        .execute(|migration| {
            migration.create_table("Test", |t| {
                t.add_column("id", types::integer().unique(true));
                t.add_column("dummy", types::integer());
                t.inject_custom("network_mac  macaddr");
                t.add_index("unique", types::index(vec!["network_mac", "dummy"]).unique(true));
                t.add_index("non_unique", types::index(vec!["network_mac", "dummy"]).unique(false));
                t.inject_custom("Primary Key (\"network_mac\", \"dummy\")");
            });
        })
        .await;

    let warnings = dbg!(api.introspection_warnings().await);
    assert_eq!(
        &warnings,
        "[{\"code\":3,\"message\":\"These fields were commented out because Prisma currently does not support their types.\",\"affected\":[{\"model\":\"Test\",\"field\":\"network_mac\",\"tpe\":\"macaddr\"}]}]"
    );

    let result = dbg!(api.introspect().await);
    assert_eq!(&result, "model Test {\n  id             Int     @unique\n  dummy          Int\n  // This type is currently not supported.\n  // network_mac macaddr\n}\n");
}

#[test_each_connector(tags("postgres"))]
async fn introspecting_a_table_with_only_an_unsupported_id_type_should_comment_it_out(api: &TestApi) {
    let barrel = api.barrel();
    let _setup_schema = barrel
        .execute(|migration| {
            migration.create_table("Test", |t| {
                t.add_column("dummy", types::integer());
                t.inject_custom("network_mac  macaddr Primary Key");
            });
        })
        .await;

    let warnings = dbg!(api.introspection_warnings().await);
    assert_eq!(
        &warnings,
        "[{\"code\":1,\"message\":\"The following models were commented out as they do not have a valid unique identifier or id. This is currently not supported by Prisma.\",\"affected\":[{\"model\":\"Test\"}]},{\"code\":3,\"message\":\"These fields were commented out because Prisma currently does not support their types.\",\"affected\":[{\"model\":\"Test\",\"field\":\"network_mac\",\"tpe\":\"macaddr\"}]}]"
    );

    let result = dbg!(api.introspect().await);
    assert_eq!(&result, "// The underlying table does not contain a valid unique identifier and can therefore currently not be handled.\n// model Test {\n  // dummy       Int\n  // This type is currently not supported.\n  // network_mac macaddr @id\n// }\n");
}

#[test_each_connector(tags("postgres"))]
async fn introspecting_an_unsupported_type_should_comment_it_out(api: &TestApi) {
    let barrel = api.barrel();
    let _setup_schema = barrel
        .execute(|migration| {
            migration.create_table("Test", |t| {
                t.add_column("id", types::primary());
                t.inject_custom("network_inet inet");
                t.inject_custom("network_mac  macaddr");
            });
        })
        .await;

    let warnings = dbg!(api.introspection_warnings().await);
    assert_eq!(
        &warnings,
        "[{\"code\":3,\"message\":\"These fields were commented out because Prisma currently does not support their types.\",\"affected\":[{\"model\":\"Test\",\"field\":\"network_mac\",\"tpe\":\"macaddr\"}]}]"
    );

    let result = dbg!(api.introspect().await);
    assert_eq!(&result, "model Test {\n  id             Int      @default(autoincrement()) @id\n  network_inet   String?\n  // This type is currently not supported.\n  // network_mac macaddr?\n}\n");
}

#[test_each_connector(tags("postgres"))]
async fn remapping_field_names_to_empty_should_comment_them_out(api: &TestApi) {
    api.barrel()
        .execute(|migration| {
            migration.create_table("User", |t| {
                t.add_column("1", types::text());
                t.add_column("last", types::primary());
            });
        })
        .await;

    let dm = "model User {\n  // This field was commented out because of an invalid name. Please provide a valid one that matches [a-zA-Z][a-zA-Z0-9_]*\n  // 1 String @map(\"1\")\n  last Int    @default(autoincrement()) @id\n}\n";

    let result = dbg!(api.introspect().await);
    assert_eq!(&result, dm);
}

// #[test_each_connector(tags("postgres"))]
// async fn introspecting_a_relation_based_on_an_unsupported_field_name_should_drop_it(api: &TestApi) {
//     let barrel = api.barrel();
//     let _setup_schema = barrel
//         .execute(|migration| {
//             migration.create_table("User", |t| {
//                 t.add_column("id", types::primary());
//                 t.inject_custom("\"1\"  integer Not null Unique");
//             });
//             migration.create_table("Post", |t| {
//                 t.add_column("id", types::primary());
//                 t.inject_custom("user_1 integer REFERENCES \"User\"(\"1\")");
//             });
//         })
//         .await;
//
//     let warnings = dbg!(api.introspection_warnings().await);
//     assert_eq!(
//         &warnings,
//         "[{\"code\":2,\"message\":\"These fields were commented out because their names are currently not supported by Prisma. Please provide valid ones that match [a-zA-Z][a-zA-Z0-9_]* using the `@map` directive.\",\"affected\":[{\"model\":\"User\",\"field\":\"1\"}]}]"
//     );
//
//     let result = dbg!(api.introspect().await);
//     assert_eq!(&result, "model Post {\n  id     Int   @default(autoincrement()) @id\n  user_1 Int?\n  User   User? @relation(fields: [user_1], references: [1])\n}\n\nmodel User {\n  id   Int    @default(autoincrement()) @id\n  // This field was commented out because of an invalid name. Please provide a valid one that matches [a-zA-Z][a-zA-Z0-9_]*\n  // 1 Int    @map(\"1\") @unique\n  Post Post[]\n}\n");
// }
