use barrel::types;
use expect_test::expect;
use sql_introspection_tests::test_api::*;
use test_macros::test_connector;

#[test_connector(tags(KingbaseMysql))]
async fn referential_actions_are_introspected(api: &mut TestApi) -> TestResult {
    api.barrel()
        .execute(|migration| {
            migration.create_table("Parent", |t| {
                t.add_column("id", types::primary());
            });

            migration.create_table("Child", |t| {
                t.add_column("id", types::primary());
                t.add_column("parent_id", types::integer().nullable(true));
                t.inject_custom(
                    "CONSTRAINT child_parent_fk FOREIGN KEY (parent_id) REFERENCES `Parent`(id) ON DELETE SET NULL ON UPDATE CASCADE",
                );
            });

            migration.create_table("RequiredChild", |t| {
                t.add_column("id", types::primary());
                t.add_column("parent_id", types::integer().nullable(false));
                t.inject_custom(
                    "CONSTRAINT required_parent_fk FOREIGN KEY (parent_id) REFERENCES `Parent`(id) ON DELETE RESTRICT ON UPDATE NO ACTION",
                );
            });
        })
        .await?;

    let expected = expect![[r#"
        model Child {
          id        Int     @id @default(autoincrement())
          parent_id Int?
          Parent    Parent? @relation(fields: [parent_id], references: [id], map: "child_parent_fk")
        }

        model Parent {
          id            Int             @id @default(autoincrement())
          Child         Child[]
          RequiredChild RequiredChild[]
        }

        model RequiredChild {
          id        Int    @id @default(autoincrement())
          parent_id Int
          Parent    Parent @relation(fields: [parent_id], references: [id], onUpdate: NoAction, map: "required_parent_fk")
        }
    "#]];

    expected.assert_eq(&api.introspect_dml().await?);

    Ok(())
}
