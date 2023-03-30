use barrel::types;
use introspection_engine_tests::{assert_eq_json, test_api::*};
use serde_json::json;

#[test_connector(tags(CockroachDb))]
async fn a_table_without_uniques_should_ignore(api: &TestApi) -> TestResult {
    api.barrel()
        .execute(|migration| {
            migration.create_table("User", |t| {
                t.add_column("id", types::primary());
            });
            migration.create_table("Post", |t| {
                t.add_column("id", types::integer());
                t.add_column("user_id", types::integer().nullable(false));
                t.add_index("Post_user_id_idx", types::index(["user_id"]));
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
          id   BigInt @id @default(autoincrement())
          Post Post[] @ignore
        }
    "#]];

    expected.assert_eq(&api.introspect_dml().await?);

    Ok(())
}

#[test_connector(tags(CockroachDb))]
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
          id   BigInt @id @default(autoincrement())
          ip   Int    @unique
          Post Post[] @ignore
        }
    "#]];

    expected.assert_eq(&api.introspect_dml().await?);

    Ok(())
}

#[test_connector(tags(CockroachDb))]
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
          id      BigInt                   @id @default(autoincrement())
          broken  Unsupported("geometry")
          broken2 Unsupported("geography")
        }
    "#]];

    let result = api.introspect_dml().await?;

    dm.assert_eq(&result);

    Ok(())
}
