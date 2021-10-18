mod mssql;
mod mysql;
mod sqlite;

use barrel::types;
use expect_test::expect;
use indoc::indoc;
use introspection_engine_tests::{assert_eq_json, test_api::*, TestResult};
use serde_json::json;
use test_macros::test_connector;

#[test_connector(exclude(Mssql, Mysql))]
async fn a_table_without_uniques_should_ignore(api: &TestApi) -> TestResult {
    api.barrel()
        .execute(|migration| {
            migration.create_table("User", |t| {
                t.add_column("id", types::primary());
            });
            migration.create_table("Post", |t| {
                t.add_column("id", types::integer());
                t.add_column("user_id", types::integer().nullable(false));
                t.add_index("Post_user_id_idx", types::index(&["user_id"]));
                t.add_foreign_key(&["user_id"], "User", &["id"]);
            });
        })
        .await?;

    let expected = expect![[r#"
        /// The underlying table does not contain a valid unique identifier and can therefore currently not be handled by the Prisma Client.
        model Post {
          id      Int
          user_id Int
          User    User @relation(fields: [user_id], references: [id], onDelete: NoAction, onUpdate: NoAction)

          @@index([user_id])
          @@ignore
        }

        model User {
          id   Int    @id @default(autoincrement())
          Post Post[] @ignore
        }
    "#]];

    expected.assert_eq(&api.introspect_dml().await?);

    Ok(())
}

#[test_connector(tags(Postgres), exclude(Cockroach))]
async fn relations_between_ignored_models_should_not_have_field_level_ignores(api: &TestApi) -> TestResult {
    api.barrel()
        .execute(|migration| {
            migration.create_table("User", |t| {
                t.inject_custom("id macaddr primary key not null");
            });
            migration.create_table("Post", |t| {
                t.inject_custom("id macaddr primary key not null");
                t.inject_custom("user_id macaddr not null");
                t.add_foreign_key(&["user_id"], "User", &["id"]);
            });
        })
        .await?;

    let expected = expect![[r#"
        /// The underlying table does not contain a valid unique identifier and can therefore currently not be handled by the Prisma Client.
        model Post {
          id      Unsupported("macaddr") @id
          user_id Unsupported("macaddr")
          User    User                   @relation(fields: [user_id], references: [id], onDelete: NoAction, onUpdate: NoAction)

          @@ignore
        }

        /// The underlying table does not contain a valid unique identifier and can therefore currently not be handled by the Prisma Client.
        model User {
          id   Unsupported("macaddr") @id
          Post Post[]

          @@ignore
        }
    "#]];

    expected.assert_eq(&api.introspect_dml().await?);

    Ok(())
}

#[test_connector(exclude(Sqlite, Mysql))]
async fn a_table_without_required_uniques(api: &TestApi) -> TestResult {
    api.barrel()
        .execute(|migration| {
            migration.create_table("Post", |t| {
                t.add_column("id", types::integer());
                t.add_column("opt_unique", types::integer().nullable(true));

                t.add_constraint("sqlite_autoindex_Post_1", types::unique_constraint(vec!["opt_unique"]));
            });
        })
        .await?;

    let expected = expect![[r#"
        /// The underlying table does not contain a valid unique identifier and can therefore currently not be handled by the Prisma Client.
        model Post {
          id         Int
          opt_unique Int? @unique(map: "sqlite_autoindex_Post_1")

          @@ignore
        }
    "#]];

    expected.assert_eq(&api.introspect_dml().await?);

    Ok(())
}

#[test_connector(exclude(Cockroach))] // there is no such thing on cockroach, you will get the rowid column
async fn a_table_without_fully_required_compound_unique(api: &TestApi) -> TestResult {
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

    let dm = expect![[r#"
        /// The underlying table does not contain a valid unique identifier and can therefore currently not be handled by the Prisma Client.
        model Post {
          id         Int
          opt_unique Int?
          req_unique Int

          @@unique([opt_unique, req_unique], map: "sqlite_autoindex_Post_1")
          @@ignore
        }
    "#]];

    let result = api.introspect_dml().await?;
    dm.assert_eq(&result);

    Ok(())
}

#[test_connector(tags(Postgres), exclude(Cockroach))]
async fn unsupported_type_keeps_its_usages(api: &TestApi) -> TestResult {
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
        "message": "These fields are not supported by the Prisma Client, because Prisma currently does not support their types.",
        "affected": [
            {
                "model": "Test",
                "field": "broken",
                "tpe": "macaddr"
            }
        ]
    }]);

    assert_eq_json!(expected, api.introspection_warnings().await?);

    let dm = expect![[r#"
        model Test {
          id     Int                    @unique
          dummy  Int
          broken Unsupported("macaddr")

          @@id([broken, dummy])
          @@unique([broken, dummy], map: "unique")
          @@index([broken, dummy], map: "non_unique")
        }
    "#]];

    let result = api.introspect_dml().await?;

    dm.assert_eq(&result);

    Ok(())
}

#[test_connector(tags(Cockroach))]
async fn unsupported_type_keeps_its_usages_cockroach(api: &TestApi) -> TestResult {
    api.barrel()
        .execute(|migration| {
            migration.create_table("Test", |t| {
                t.add_column("id", types::primary());
                // Geometry/Geography is the only type that is not supported by Prisma, but is also not
                // indexable (only inverted-indexable).
                t.add_column("broken", types::custom("geometry"));
                t.add_column("broken2", types::custom("geography"));
            });
        })
        .await?;

    let expected = json!([{
        "code": 3,
        "message": "These fields are not supported by the Prisma Client, because Prisma currently does not support their types.",
        "affected": [
            {
                "model": "Test",
                "field": "broken",
                "tpe": "geometry"
            },
            {
                "model": "Test",
                "field": "broken2",
                "tpe": "geography"
            },
        ]
    }]);

    assert_eq_json!(expected, api.introspection_warnings().await?);

    let dm = expect![[r#"
        model Test {
          id      Int                      @id @default(autoincrement())
          broken  Unsupported("geometry")
          broken2 Unsupported("geography")
        }
    "#]];

    let result = api.introspect_dml().await?;

    dm.assert_eq(&result);

    Ok(())
}

#[test_connector(tags(Postgres), exclude(Cockroach))]
async fn a_table_with_only_an_unsupported_id(api: &TestApi) -> TestResult {
    api.barrel()
        .execute(|migration| {
            migration.create_table("Test", |t| {
                t.add_column("dummy", types::integer());
                t.add_column(
                    "network_mac",
                    types::custom("macaddr").primary(true).default("08:00:2b:01:02:03"),
                );
            });
        })
        .await?;

    let expected = json!([
        {
            "code": 1,
            "message": "The following models were commented out as they do not have a valid unique identifier or id. This is currently not supported by the Prisma Client.",
            "affected": [{
                "model": "Test"
            }]
        },
        {
            "code": 3,
            "message": "These fields are not supported by the Prisma Client, because Prisma currently does not support their types.",
            "affected": [{
                "model": "Test",
                "field": "network_mac",
                "tpe": "macaddr"
            }]
        }
    ]);

    assert_eq_json!(expected, api.introspection_warnings().await?);

    let dm = indoc! {r#"
        /// The underlying table does not contain a valid unique identifier and can therefore currently not be handled by the Prisma Client.
        model Test {
          dummy       Int
          network_mac Unsupported("macaddr") @id @default(dbgenerated("'08:00:2b:01:02:03'::macaddr"))

          @@ignore
        }
    "#};

    api.assert_eq_datamodels(dm, &api.introspect().await?);

    Ok(())
}

#[test_connector(tags(Postgres), exclude(Cockroach))]
async fn a_table_with_unsupported_types_in_a_relation(api: &TestApi) -> TestResult {
    api.barrel()
        .execute(|migration| {
            migration.create_table("User", |t| {
                t.add_column("id", types::primary());
                t.inject_custom("ip cidr not null unique");
            });
            migration.create_table("Post", |t| {
                t.add_column("id", types::primary());
                t.inject_custom("user_ip cidr not null");
                t.add_foreign_key(&["user_ip"], "User", &["ip"]);
            });
        })
        .await?;

    let expected = expect![[r#"
        model Post {
          id      Int                 @id @default(autoincrement())
          user_ip Unsupported("cidr")
          User    User                @relation(fields: [user_ip], references: [ip], onDelete: NoAction, onUpdate: NoAction)
        }

        model User {
          id   Int                 @id @default(autoincrement())
          ip   Unsupported("cidr") @unique
          Post Post[]
        }
    "#]];

    expected.assert_eq(&api.introspect_dml().await?);

    Ok(())
}

#[test_connector]
async fn remapping_field_names_to_empty(api: &TestApi) -> TestResult {
    api.barrel()
        .execute(|migration| {
            migration.create_table("User", |t| {
                t.add_column("1", types::text());
                t.add_column("last", types::integer().increments(true));

                t.add_constraint("User_pkey", types::primary_constraint(vec!["last"]));
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

    api.assert_eq_datamodels(dm, &api.introspect().await?);

    Ok(())
}

#[test_connector(tags(Postgres), exclude(Cockroach))]
async fn dbgenerated_in_unsupported(api: &TestApi) -> TestResult {
    api.barrel()
        .execute(|migration| {
            migration.create_table("Blog", move |t| {
                t.add_column("id", types::primary());
                t.inject_custom("number Integer Default 1");
                t.inject_custom("bigger_number Integer DEFAULT sqrt(4)");
                t.inject_custom("point Point DEFAULT Point(0, 0)");
            });
        })
        .await?;

    let dm = indoc! {r##"
        model Blog {
          id                Int    @id @default(autoincrement())
          number            Int?   @default(1)
          bigger_number     Int?   @default(dbgenerated("sqrt((4)::double precision)"))
          point             Unsupported("point")? @default(dbgenerated("point((0)::double precision, (0)::double precision)"))
        }
    "##};

    api.assert_eq_datamodels(dm, &api.introspect().await?);

    Ok(())
}

#[test_connector(tags(Postgres))]
async fn commenting_out_a_table_without_columns(api: &TestApi) -> TestResult {
    api.barrel()
        .execute(|migration| {
            migration.create_table("Test", |_t| {});
        })
        .await?;

    let expected = json!([{
        "code": 14,
        "message": "The following models were commented out as we could not retrieve columns for them. Please check your privileges.",
        "affected": [
            {
                "model": "Test"
            }
        ]
    }]);

    assert_eq_json!(expected, api.introspection_warnings().await?);

    let dm = indoc! {r#"
        // We could not retrieve columns for the underlying table. Either it has none or you are missing rights to see them. Please check your privileges.
        // model Test {
        // }
    "#};

    api.assert_eq_datamodels(dm, &api.introspect().await?);

    Ok(())
}

#[test_connector(tags(Postgres))]
async fn ignore_on_back_relation_field_if_pointing_to_ignored_model(api: &TestApi) -> TestResult {
    api.barrel()
        .execute(|migration| {
            migration.create_table("User", |t| {
                t.add_column("id", types::primary());
                t.inject_custom("ip integer not null unique");
            });
            migration.create_table("Post", |t| {
                t.add_column("id", types::integer());
                t.inject_custom("user_ip integer not null ");
                t.add_foreign_key(&["user_ip"], "User", &["ip"]);
            });
        })
        .await?;

    let expected = expect![[r#"
        /// The underlying table does not contain a valid unique identifier and can therefore currently not be handled by the Prisma Client.
        model Post {
          id      Int
          user_ip Int
          User    User @relation(fields: [user_ip], references: [ip], onDelete: NoAction, onUpdate: NoAction)

          @@ignore
        }

        model User {
          id   Int    @id @default(autoincrement())
          ip   Int    @unique
          Post Post[] @ignore
        }
    "#]];

    expected.assert_eq(&api.introspect_dml().await?);

    Ok(())
}
