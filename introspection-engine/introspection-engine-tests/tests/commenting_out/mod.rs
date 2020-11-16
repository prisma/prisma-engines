use barrel::types;
use indoc::indoc;
use introspection_engine_tests::{assert_eq_json, test_api::*};
use pretty_assertions::assert_eq;
use serde_json::json;
use test_macros::test_each_connector_mssql as test_each_connector;

#[test_each_connector]
async fn a_table_without_uniques(api: &TestApi) -> crate::TestResult {
    api.barrel()
        .execute(|migration| {
            migration.create_table("User", |t| {
                t.add_column("id", types::primary());
            });
            migration.create_table("Post", |t| {
                t.add_column("id", types::integer());
                t.add_column("user_id", types::integer().nullable(false));
                t.add_foreign_key(&["user_id"], "User", &["id"]);
            });
        })
        .await?;

    let dm = if api.sql_family().is_mysql() {
        indoc! {r#"
            // The underlying table does not contain a valid unique identifier and can therefore currently not be handled.
            // model Post {
              // id      Int
              // user_id Int
              // User    User @relation(fields: [user_id], references: [id])

              // @@index([user_id], name: "user_id")
            // }

            model User {
              id      Int    @id @default(autoincrement())
              // Post Post[]
            }
        "#}
    } else {
        indoc! {r#"
            // The underlying table does not contain a valid unique identifier and can therefore currently not be handled.
            // model Post {
              // id      Int
              // user_id Int
              // User    User @relation(fields: [user_id], references: [id])
            // }

            model User {
              id      Int    @id @default(autoincrement())
              // Post Post[]
            }
        "#}
    };

    assert_eq!(dm, &api.introspect().await?);

    Ok(())
}

#[test_each_connector]
async fn a_table_without_required_uniques(api: &TestApi) -> crate::TestResult {
    api.barrel()
        .execute(|migration| {
            migration.create_table("Post", |t| {
                t.add_column("id", types::integer());
                t.add_column("opt_unique", types::integer().unique(true).nullable(true));
            });
        })
        .await?;

    let dm = indoc! {r#"
        // The underlying table does not contain a valid unique identifier and can therefore currently not be handled.
        // model Post {
          // id         Int
          // opt_unique Int? @unique
        // }
    "#};

    assert_eq!(dm, &api.introspect().await?);

    Ok(())
}

#[test_each_connector]
async fn a_table_without_fully_required_compound_unique(api: &TestApi) -> crate::TestResult {
    api.barrel()
        .execute(|migration| {
            migration.create_table("Post", |t| {
                t.add_column("id", types::integer());
                t.add_column("opt_unique", types::integer().nullable(true));
                t.add_column("req_unique", types::integer().nullable(false));

                t.add_constraint(
                    "sqlite_autoindex_Post_1",
                    types::unique_constraint(vec!["opt_unique", "req_unique"]),
                );
            });
        })
        .await?;

    let dm = indoc! {r#"
        // The underlying table does not contain a valid unique identifier and can therefore currently not be handled.
        // model Post {
          // id         Int
          // opt_unique Int?
          // req_unique Int

          // @@unique([opt_unique, req_unique], name: "sqlite_autoindex_Post_1")
        // }
    "#};

    assert_eq!(dm, &api.introspect().await?);

    Ok(())
}

#[test_each_connector(tags("postgres"))]
async fn commenting_out_an_unsupported_type_drops_its_usages(api: &TestApi) -> crate::TestResult {
    api.barrel()
        .execute(|migration| {
            migration.create_table("Test", |t| {
                t.add_column("id", types::integer().unique(true));
                t.add_column("dummy", types::integer());
                t.add_column("broken", types::custom("macaddr"));

                t.add_index("unique", types::index(vec!["broken", "dummy"]).unique(true));
                t.add_index("non_unique", types::index(vec!["broken", "dummy"]).unique(false));
                t.set_primary_key(&["broken", "dummy"]);
            });
        })
        .await?;

    let expected = json!([{
        "code": 3,
        "message": "These fields were commented out because Prisma currently does not support their types.",
        "affected": [
            {
                "model": "Test",
                "field": "broken",
                "tpe": "macaddr"
            }
        ]
    }]);

    assert_eq_json!(expected, api.introspection_warnings().await?);

    let dm = indoc! {r#"
        model Test {
          id        Int     @unique
          dummy     Int
          // This type is currently not supported.
          // broken macaddr
        }
    "#};

    assert_eq!(dm, &api.introspect().await?);

    Ok(())
}

#[test_each_connector(tags("postgres"))]
async fn a_table_with_only_an_unsupported_id(api: &TestApi) -> crate::TestResult {
    api.barrel()
        .execute(|migration| {
            migration.create_table("Test", |t| {
                t.add_column("dummy", types::integer());
                t.add_column("network_mac", types::custom("macaddr").primary(true));
            });
        })
        .await?;

    let expected = json!([
        {
            "code": 1,
            "message": "The following models were commented out as they do not have a valid unique identifier or id. This is currently not supported by Prisma.",
            "affected": [{
                "model": "Test"
            }]
        },
        {
            "code": 3,
            "message": "These fields were commented out because Prisma currently does not support their types.",
            "affected": [{
                "model": "Test",
                "field": "network_mac",
                "tpe": "macaddr"
            }]
        }
    ]);

    assert_eq_json!(expected, api.introspection_warnings().await?);

    let dm = indoc! {r#"
        // The underlying table does not contain a valid unique identifier and can therefore currently not be handled.
        // model Test {
          // dummy       Int
          // This type is currently not supported.
          // network_mac macaddr @id
        // }
    "#};

    assert_eq!(dm, &api.introspect().await?);

    Ok(())
}

#[test_each_connector]
async fn remapping_field_names_to_empty(api: &TestApi) -> crate::TestResult {
    api.barrel()
        .execute(|migration| {
            migration.create_table("User", |t| {
                t.add_column("1", types::text());
                t.add_column("last", types::primary());
            });
        })
        .await?;

    let dm = indoc! {r#"
        model User {
          // This field was commented out because of an invalid name. Please provide a valid one that matches [a-zA-Z][a-zA-Z0-9_]*
          // 1 String @map("1")
          last Int    @id @default(autoincrement())
        }
    "#};

    assert_eq!(dm, &api.introspect().await?);

    Ok(())
}

#[test_each_connector(tags("postgres"))]
async fn db_genererated_values_should_add_comments(api: &TestApi) -> crate::TestResult {
    api.barrel()
        .execute_with_schema(
            |migration| {
                migration.create_table("Blog", move |t| {
                    t.add_column("id", types::primary());
                    t.inject_custom("number Integer Default 1");
                    t.inject_custom("bigger_number Integer DEFAULT sqrt(4)");
                    t.inject_custom("point Point DEFAULT Point(0, 0)");
                });
            },
            api.schema_name(),
        )
        .await?;

    let dm = indoc! {r##"
        model Blog {
          id            Int    @id @default(autoincrement())
          number        Int?   @default(1)
          /// This field's default value can currently not be parsed: `sqrt((4)::double precision)`.
          bigger_number Int?   @default(dbgenerated())
          // This type is currently not supported.
          // This field's default value can currently not be parsed: `point((0)::double precision, (0)::double precision)`.
          // point      point? @default(dbgenerated())
        }
    "##};

    assert_eq!(dm, &api.introspect().await?);

    Ok(())
}
