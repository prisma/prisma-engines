use indoc::indoc;
use sql_introspection_tests::test_api::*;
use test_macros::test_connector;

#[test_connector(tags(KingbaseMysql))]
async fn check_constraints_are_introspected(api: &mut TestApi) -> TestResult {
    api.raw_cmd(indoc! {r#"
        CREATE TABLE t1(
            id integer NOT NULL PRIMARY KEY,
            CHECK (c1 <> c2),
            c1 INT CHECK (c1 > 10),
            c2 INT CONSTRAINT c2_positive CHECK (c2 > 0),
            c3 INT CHECK (c3 < 100),
            CONSTRAINT c1_nonzero CHECK (c1 <> 0),
            CHECK (c1 > c3)
        );

        CREATE TABLE some_user (
            user_id integer NOT NULL PRIMARY KEY
        );
    "#})
        .await;

    let schema = expect![[r#"
        generator client {
          provider = "prisma-client"
        }

        datasource db {
          provider = "kingbase-mysql"
        }

        model some_user {
          user_id Int @id
        }

        /// This table contains check constraints and requires additional setup for migrations. Visit https://pris.ly/d/check-constraints for more info.
        model t1 {
          id Int  @id
          c1 Int?
          c2 Int?
          c3 Int?
        }
    "#]];

    api.expect_datamodel(&schema).await;
    psl::parse_schema_without_extensions(schema.data()).unwrap();

    let warnings = expect![[r#"
        *** WARNING ***

        These constraints are not supported by Prisma Client, because Prisma currently does not fully support check constraints. Read more: https://pris.ly/d/check-constraints
          - Model: "t1", constraint: "c1_nonzero"
          - Model: "t1", constraint: "c2_positive"
          - Model: "t1", constraint: "t1_c1_check"
          - Model: "t1", constraint: "t1_c3_check"
          - Model: "t1", constraint: "t1_check"
          - Model: "t1", constraint: "t1_check1"
    "#]];

    api.expect_warnings(&warnings).await;

    let input = indoc! {r#"
        /// This table contains check constraints and requires additional setup for migrations. Visit https://pris.ly/d/kingbase-mysql-check-constraints for more info.
        model t1 {
          id Int  @id
          c1 Int?
          c2 Int?
          c3 Int?
        }

        model some_user {
          user_id Int @id
        }
    "#};

    let expected = expect![[r#"
        /// This table contains check constraints and requires additional setup for migrations. Visit https://pris.ly/d/kingbase-mysql-check-constraints for more info.
        model t1 {
          id Int  @id
          c1 Int?
          c2 Int?
          c3 Int?
        }

        model some_user {
          user_id Int @id
        }
    "#]];

    api.expect_re_introspected_datamodel(input, expected).await;

    Ok(())
}
