//! https://www.notion.so/prismaio/Indexes-Constraints-Check-constraints-MySQL-e56a210937904cae91836e202830202a

use indoc::indoc;
use sql_introspection_tests::test_api::*;
use test_macros::test_connector;

// Note: MySQL 5.6 and 5.7 do not support check constraints, so this test is only run on MySQL 8.0.
#[test_connector(tags(Mysql8), exclude(Vitess))]
async fn check_constraints_stopgap(api: &mut TestApi) -> TestResult {
    let raw_sql = indoc! {r#"
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
    "#};

    api.raw_cmd(raw_sql).await;

    let schema = expect![[r#"
        generator client {
          provider = "prisma-client-js"
        }

        datasource db {
          provider = "mysql"
          url      = "env(TEST_DATABASE_URL)"
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

    // ensure the introspected schema is valid
    psl::parse_schema(schema.data()).unwrap();

    let expectation = expect![[r#"
        *** WARNING ***

        These constraints are not supported by Prisma Client, because Prisma currently does not fully support check constraints. Read more: https://pris.ly/d/check-constraints
          - Model: "t1", constraint: "c1_nonzero"
          - Model: "t1", constraint: "c2_positive"
          - Model: "t1", constraint: "t1_chk_1"
          - Model: "t1", constraint: "t1_chk_2"
          - Model: "t1", constraint: "t1_chk_3"
          - Model: "t1", constraint: "t1_chk_4"
    "#]];

    api.expect_warnings(&expectation).await;

    let input = indoc! { r#"
        /// This table contains check constraints and requires additional setup for migrations. Visit https://pris.ly/d/mysql-check-constraints for more info.
        model t1 {
          id Int  @id
          c1 Int?
          c2 Int?
          c3 Int?
        }

        model some_user {
          user_id Int @id
        }
      "#
    };

    let expectation = expect![[r#"
        /// This table contains check constraints and requires additional setup for migrations. Visit https://pris.ly/d/mysql-check-constraints for more info.
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
    api.expect_re_introspected_datamodel(input, expectation).await;

    Ok(())
}
