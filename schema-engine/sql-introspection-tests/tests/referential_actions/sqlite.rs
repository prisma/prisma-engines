use barrel::types;
use sql_introspection_tests::test_api::*;

#[test_connector(tags(Sqlite))]
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
          id Int @id @default(autoincrement())
          b  b[]
        }

        model b {
          id   Int @id @default(autoincrement())
          a_id Int
          a    a   @relation(fields: [a_id], references: [id], onDelete: Cascade, onUpdate: NoAction)
        }
    "#]];

    expected.assert_eq(&api.introspect_dml().await?);

    Ok(())
}
