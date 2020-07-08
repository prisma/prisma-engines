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

    let dm = "model User {\n  id Int @default(autoincrement()) @id\n}\n\n// The underlying table does not contain a unique identifier and can therefore currently not be handled.\n// model Post {\n  // id      Int\n  // user_id Int\n  // User    User @relation(fields: [user_id], references: [id])\n// }\n";

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

    let dm = "// The underlying table does not contain a unique identifier and can therefore currently not be handled.\n// model Post {\n  // id      Int\n  // user_id Int\n  // User    User @relation(fields: [user_id], references: [id])\n\n  // @@index([user_id], name: \"user_id\")\n// }\n\nmodel User {\n  id Int @default(autoincrement()) @id\n}\n";

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
