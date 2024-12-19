use barrel::types;
use sql_introspection_tests::test_api::*;

#[test_connector(tags(CockroachDb))]
async fn default_referential_actions_with_restrict(api: &mut TestApi) -> TestResult {
    api.barrel()
        .execute(|migration| {
            migration.create_table("a", |t| {
                t.add_column("id", types::primary());
            });

            migration.create_table("b", |t| {
                t.add_column("id", types::primary());
                t.add_column("a_id", types::integer().nullable(false));
                t.inject_custom(
                    "CONSTRAINT asdf FOREIGN KEY (a_id) REFERENCES a(id) ON DELETE RESTRICT ON UPDATE CASCADE",
                );
            });
        })
        .await?;

    let expected = expect![[r#"
        model a {
          id BigInt @id @default(autoincrement())
          b  b[]
        }

        model b {
          id   BigInt @id @default(autoincrement())
          a_id BigInt
          a    a      @relation(fields: [a_id], references: [id], map: "asdf")
        }
    "#]];

    expected.assert_eq(&api.introspect_dml().await?);

    Ok(())
}

#[test_connector(tags(CockroachDb))]
async fn referential_actions(api: &mut TestApi) -> TestResult {
    api.barrel()
        .execute(move |migration| {
            migration.create_table("a", |t| {
                t.add_column("id", types::primary());
            });

            migration.create_table("b", move |t| {
                t.add_column("id", types::primary());
                t.add_column("a_id", types::integer().nullable(false));

                t.inject_custom(
                    "CONSTRAINT asdf FOREIGN KEY (a_id) REFERENCES a(id) ON DELETE CASCADE ON UPDATE NO ACTION",
                );
            });
        })
        .await?;

    let expected = expect![[r#"
        model a {
          id BigInt @id @default(autoincrement())
          b  b[]
        }

        model b {
          id   BigInt @id @default(autoincrement())
          a_id BigInt
          a    a      @relation(fields: [a_id], references: [id], onDelete: Cascade, onUpdate: NoAction, map: "asdf")
        }
    "#]];

    expected.assert_eq(&api.introspect_dml().await?);

    Ok(())
}
