//! https://www.notion.so/prismaio/PostgreSQL-Exclusion-Constraints-fb2ecc44f773463f908d3d0e2d737271

use indoc::indoc;
use sql_introspection_tests::test_api::*;
use test_macros::test_connector;

#[test_connector(tags(CockroachDb))]
async fn aragon_test_cockroachdb(api: &mut TestApi) -> TestResult {
    let raw_sql = indoc! {r#"
        CREATE TABLE users (
            user_id INT8 PRIMARY KEY
        );
        
        CREATE TABLE tokens (
            token_id INT8 PRIMARY KEY,
            token_scope STRING NULL,
            CONSTRAINT tokens_token_scope_check CHECK (token_scope = ANY ARRAY['MAGICLINK':::STRING, 'API':::STRING]:::STRING[])
        );
    "#};

    api.raw_cmd(raw_sql).await;

    let schema = expect![[r#"
        generator client {
          provider = "prisma-client-js"
        }

        datasource db {
          provider = "cockroachdb"
          url      = "env(TEST_DATABASE_URL)"
        }

        /// This table contains check constraints and requires additional setup for migrations. Visit https://pris.ly/d/postgres-check-constraints for more info.
        model tokens {
          token_id    BigInt  @id
          token_scope String?
        }

        model users {
          user_id BigInt @id
        }
    "#]];

    api.expect_datamodel(&schema).await;

    let expectation = expect![[r#"
        *** WARNING ***

        These constraints are not supported by the Prisma Client, because Prisma currently does not fully support check constraints. Read more: https://pris.ly/d/postgres-check-constraints
          - Model: "tokens", constraint: "tokens_token_scope_check"
    "#]];

    api.expect_warnings(&expectation).await;

    Ok(())
}

#[test_connector(tags(CockroachDb))]
async fn noalyss_folder_test_cockroachdb(api: &mut TestApi) -> TestResult {
    let raw_sql = indoc! {r#"
        CREATE SEQUENCE user_active_security_id_seq MINVALUE 1 MAXVALUE 2147483647 INCREMENT 1 START 1;

        CREATE TABLE user_active_security (
            id INT8 NOT NULL DEFAULT nextval('user_active_security_id_seq'::REGCLASS),
            us_login STRING NOT NULL,
            us_ledger VARCHAR(1) NOT NULL,
            us_action VARCHAR(1) NOT NULL,
            CONSTRAINT user_active_security_pk PRIMARY KEY (id ASC),
            CONSTRAINT user_active_security_action_check CHECK (us_action::STRING = ANY ARRAY['Y':::STRING::VARCHAR::STRING, 'N':::STRING::VARCHAR::STRING]:::STRING[]),
            CONSTRAINT user_active_security_ledger_check CHECK (us_ledger::STRING = ANY ARRAY['Y':::STRING::VARCHAR::STRING, 'N':::STRING::VARCHAR::STRING]:::STRING[])
        );

        COMMENT ON COLUMN user_active_security.us_login IS e'user\'s login';
        COMMENT ON COLUMN user_active_security.us_ledger IS 'Flag Security for ledger';
        COMMENT ON COLUMN user_active_security.us_action IS 'Security for action';
    "#};

    api.raw_cmd(raw_sql).await;

    let schema = expect![[r#"
        generator client {
          provider = "prisma-client-js"
        }

        datasource db {
          provider = "cockroachdb"
          url      = "env(TEST_DATABASE_URL)"
        }

        /// This table contains check constraints and requires additional setup for migrations. Visit https://pris.ly/d/postgres-check-constraints for more info.
        /// This model or at least one of its fields has comments in the database, and requires an additional setup for migrations: Read more: https://pris.ly/d/database-comments
        model user_active_security {
          id        BigInt @id(map: "user_active_security_pk") @default(sequence(maxValue: 2147483647))
          us_login  String
          us_ledger String @db.String(1)
          us_action String @db.String(1)
        }
    "#]];

    api.expect_datamodel(&schema).await;

    let expectation = expect![[r#"
        *** WARNING ***

        These constraints are not supported by the Prisma Client, because Prisma currently does not fully support check constraints. Read more: https://pris.ly/d/postgres-check-constraints
          - Model: "user_active_security", constraint: "user_active_security_action_check"
          - Model: "user_active_security", constraint: "user_active_security_ledger_check"

        These objects have comments defined in the database, which is not yet fully supported. Read more: https://pris.ly/d/database-comments
          - Type: "field", name: "user_active_security.us_login"
          - Type: "field", name: "user_active_security.us_ledger"
          - Type: "field", name: "user_active_security.us_action"
    "#]];

    api.expect_warnings(&expectation).await;

    Ok(())
}
