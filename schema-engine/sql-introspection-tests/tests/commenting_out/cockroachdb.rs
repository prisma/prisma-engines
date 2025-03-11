use barrel::types;
use sql_introspection_tests::test_api::*;

#[test_connector(tags(CockroachDb))]
async fn a_table_without_uniques_should_ignore(api: &mut TestApi) -> TestResult {
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
        /// The underlying table does not contain a valid unique identifier and can therefore currently not be handled by Prisma Client.
        model Post {
          id      BigInt
          user_id BigInt
          User    User   @relation(fields: [user_id], references: [id], onDelete: NoAction, onUpdate: NoAction)

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
async fn ignore_on_back_relation_field_if_pointing_to_ignored_model(api: &mut TestApi) -> TestResult {
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
        /// The underlying table does not contain a valid unique identifier and can therefore currently not be handled by Prisma Client.
        model Post {
          id      BigInt
          user_ip BigInt
          User    User   @relation(fields: [user_ip], references: [ip], onDelete: NoAction, onUpdate: NoAction)

          @@ignore
        }

        model User {
          id   BigInt @id @default(autoincrement())
          ip   BigInt @unique
          Post Post[] @ignore
        }
    "#]];

    expected.assert_eq(&api.introspect_dml().await?);

    Ok(())
}

#[test_connector(tags(CockroachDb))]
async fn unsupported_type_keeps_its_usages_cockroach(api: &mut TestApi) -> TestResult {
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

    let expected = expect![[r#"
        *** WARNING ***

        These fields are not supported by Prisma Client, because Prisma currently does not support their types:
          - Model: "Test", field: "broken", original data type: "geometry"
          - Model: "Test", field: "broken2", original data type: "geography"
    "#]];

    api.expect_warnings(&expected).await;

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
