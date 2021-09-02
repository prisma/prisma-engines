use expect_test::expect;
use indoc::indoc;
use introspection_engine_tests::test_api::*;
use quaint::prelude::Queryable;
use test_macros::test_connector;

#[test_connector(tags(Postgres))]
async fn multiple_foreign_key_constraints_are_taken_always_in_the_same_order(api: &TestApi) -> TestResult {
    let migration = indoc! {r#"
        CREATE TABLE "A"
        (
            id  int primary key,
            foo int not null
        );

        CREATE TABLE "B"
        (
            id int primary key
        );

        ALTER TABLE "A" ADD CONSTRAINT "fk_1" FOREIGN KEY (foo) REFERENCES "B"(id) ON DELETE CASCADE ON UPDATE CASCADE;
        ALTER TABLE "A" ADD CONSTRAINT "fk_2" FOREIGN KEY (foo) REFERENCES "B"(id) ON DELETE RESTRICT ON UPDATE RESTRICT;
    "#};

    api.database().raw_cmd(migration).await?;

    let expected = expect![[r#"
        model A {
          id  Int @id
          foo Int
          B   B   @relation(fields: [foo], references: [id], onDelete: Cascade)
        }

        model B {
          id Int @id
          A  A[]
        }
    "#]];

    for _ in 0..10 {
        expected.assert_eq(&api.introspect_dml().await?);
    }

    Ok(())
}
