//! https://www.notion.so/prismaio/PostgreSQL-Exclusion-Constraints-fb2ecc44f773463f908d3d0e2d737271

use indoc::indoc;
use sql_introspection_tests::test_api::*;
use test_macros::test_connector;

#[test_connector(tags(Postgres), exclude(CockroachDb))]
async fn aragon_test_postgres(api: &mut TestApi) -> TestResult {
    let raw_sql = indoc! {r#"
        CREATE TABLE tokens (
            token_id integer NOT NULL PRIMARY KEY,
            token_scope text,
            CONSTRAINT tokens_token_scope_check CHECK ((token_scope = ANY (ARRAY['MAGICLINK'::text, 'API'::text])))
        );

        CREATE TABLE users (
            user_id integer NOT NULL PRIMARY KEY
        );
    "#};

    api.raw_cmd(raw_sql).await;

    let schema = expect![[r#"
        generator client {
          provider = "prisma-client"
        }

        datasource db {
          provider = "postgresql"
          url      = "env(TEST_DATABASE_URL)"
        }

        /// This table contains check constraints and requires additional setup for migrations. Visit https://pris.ly/d/check-constraints for more info.
        model tokens {
          token_id    Int     @id
          token_scope String?
        }

        model users {
          user_id Int @id
        }
    "#]];

    api.expect_datamodel(&schema).await;

    let expectation = expect![[r#"
        *** WARNING ***

        These constraints are not supported by Prisma Client, because Prisma currently does not fully support check constraints. Read more: https://pris.ly/d/check-constraints
          - Model: "tokens", constraint: "tokens_token_scope_check"
    "#]];

    api.expect_warnings(&expectation).await;

    Ok(())
}

#[test_connector(tags(Postgres), exclude(CockroachDb))]
async fn noalyss_folder_test_postgres(api: &mut TestApi) -> TestResult {
    let raw_sql = indoc! {r"
        CREATE TABLE user_active_security (
            id BIGSERIAL NOT NULL,
            us_login TEXT NOT NULL,
            us_ledger VARCHAR(1) NOT NULL,
            us_action VARCHAR(1) NOT NULL,
            CONSTRAINT user_active_security_pk PRIMARY KEY (id),
            CONSTRAINT user_active_security_action_check CHECK (us_action::TEXT = ANY (ARRAY['Y', 'N'])),
            CONSTRAINT user_active_security_ledger_check CHECK (us_ledger::TEXT = ANY (ARRAY['Y', 'N']))
        );

        COMMENT ON COLUMN user_active_security.us_login IS e'user\'s login';
        COMMENT ON COLUMN user_active_security.us_ledger IS 'Flag Security for ledger';
        COMMENT ON COLUMN user_active_security.us_action IS 'Security for action';

        CREATE TABLE user_sec_action_profile (
          ua_id BIGSERIAL PRIMARY KEY,
          ua_right CHAR(1) CHECK (ua_right IN ('R', 'W'))
        );

        CREATE TABLE todo_list (
          tl_id BIGSERIAL PRIMARY KEY,
          is_public CHAR(1) NOT NULL DEFAULT 'N' CHECK (is_public IN ('Y', 'N'))
      );
    "};

    api.raw_cmd(raw_sql).await;

    let schema = expect![[r#"
        generator client {
          provider = "prisma-client"
        }

        datasource db {
          provider = "postgresql"
          url      = "env(TEST_DATABASE_URL)"
        }

        /// This table contains check constraints and requires additional setup for migrations. Visit https://pris.ly/d/check-constraints for more info.
        model todo_list {
          tl_id     BigInt @id @default(autoincrement())
          is_public String @default("N") @db.Char(1)
        }

        /// This table contains check constraints and requires additional setup for migrations. Visit https://pris.ly/d/check-constraints for more info.
        /// This model or at least one of its fields has comments in the database, and requires an additional setup for migrations: Read more: https://pris.ly/d/database-comments
        model user_active_security {
          id        BigInt @id(map: "user_active_security_pk") @default(autoincrement())
          us_login  String
          us_ledger String @db.VarChar(1)
          us_action String @db.VarChar(1)
        }

        /// This table contains check constraints and requires additional setup for migrations. Visit https://pris.ly/d/check-constraints for more info.
        model user_sec_action_profile {
          ua_id    BigInt  @id @default(autoincrement())
          ua_right String? @db.Char(1)
        }
    "#]];

    api.expect_datamodel(&schema).await;

    let expectation = expect![[r#"
        *** WARNING ***

        These constraints are not supported by Prisma Client, because Prisma currently does not fully support check constraints. Read more: https://pris.ly/d/check-constraints
          - Model: "todo_list", constraint: "todo_list_is_public_check"
          - Model: "user_active_security", constraint: "user_active_security_action_check"
          - Model: "user_active_security", constraint: "user_active_security_ledger_check"
          - Model: "user_sec_action_profile", constraint: "user_sec_action_profile_ua_right_check"

        These objects have comments defined in the database, which is not yet fully supported. Read more: https://pris.ly/d/database-comments
          - Type: "field", name: "user_active_security.us_login"
          - Type: "field", name: "user_active_security.us_ledger"
          - Type: "field", name: "user_active_security.us_action"
    "#]];

    api.expect_warnings(&expectation).await;

    Ok(())
}

#[test_connector(tags(Postgres), exclude(CockroachDb))]
async fn check_and_exclusion_constraints_stopgap(api: &mut TestApi) -> TestResult {
    let raw_sql = indoc! {r#"
        CREATE EXTENSION btree_gist;

        CREATE TABLE room_reservation (
            room_reservation_id serial PRIMARY KEY,
            room_id integer NOT NULL, -- this could e.g. be a foreign key to a `room` table
            reserved_at timestamptz NOT NULL,
            reserved_until timestamptz NOT NULL,
            canceled boolean DEFAULT false,
            price numeric CHECK (price > 0),
            EXCLUDE USING gist (
                room_id WITH =, tstzrange(reserved_at, reserved_until) WITH &&
            ) WHERE (NOT canceled)
        );
    "#};

    api.raw_cmd(raw_sql).await;

    let schema = expect![[r#"
        generator client {
          provider = "prisma-client"
        }

        datasource db {
          provider = "postgresql"
          url      = "env(TEST_DATABASE_URL)"
        }

        /// This table contains check constraints and requires additional setup for migrations. Visit https://pris.ly/d/check-constraints for more info.
        /// This table contains exclusion constraints and requires additional setup for migrations. Visit https://pris.ly/d/exclusion-constraints for more info.
        model room_reservation {
          room_reservation_id Int      @id @default(autoincrement())
          room_id             Int
          reserved_at         DateTime @db.Timestamptz(6)
          reserved_until      DateTime @db.Timestamptz(6)
          canceled            Boolean? @default(false)
          price               Decimal? @db.Decimal
        }
    "#]];

    api.expect_datamodel(&schema).await;

    // ensure the introspected schema is valid
    psl::parse_schema_without_extensions(schema.data()).unwrap();

    let expectation = expect![[r#"
        *** WARNING ***

        These constraints are not supported by Prisma Client, because Prisma currently does not fully support check constraints. Read more: https://pris.ly/d/check-constraints
          - Model: "room_reservation", constraint: "room_reservation_price_check"

        These constraints are not supported by Prisma Client, because Prisma currently does not fully support exclusion constraints. Read more: https://pris.ly/d/exclusion-constraints
          - Model: "room_reservation", constraint: "room_reservation_room_id_tstzrange_excl"
    "#]];

    api.expect_warnings(&expectation).await;

    let input = indoc! { r#"
        /// This table contains check constraints and requires additional setup for migrations. Visit https://pris.ly/d/check-constraints for more info.
        /// This table contains exclusion constraints and requires additional setup for migrations. Visit https://pris.ly/d/exclusion-constraints for more info.
        model room_reservation {
          room_reservation_id Int      @id @default(autoincrement())
          room_id             Int
          reserved_at         DateTime @db.Timestamptz(6)
          reserved_until      DateTime @db.Timestamptz(6)
          canceled            Boolean? @default(false)
          price               Decimal? @db.Decimal
        }
    "#};

    let expectation = expect![[r#"
        /// This table contains check constraints and requires additional setup for migrations. Visit https://pris.ly/d/check-constraints for more info.
        /// This table contains exclusion constraints and requires additional setup for migrations. Visit https://pris.ly/d/exclusion-constraints for more info.
        model room_reservation {
          room_reservation_id Int      @id @default(autoincrement())
          room_id             Int
          reserved_at         DateTime @db.Timestamptz(6)
          reserved_until      DateTime @db.Timestamptz(6)
          canceled            Boolean? @default(false)
          price               Decimal? @db.Decimal
        }
    "#]];

    api.expect_re_introspected_datamodel(input, expectation).await;

    Ok(())
}

#[test_connector(tags(Postgres), exclude(CockroachDb))]
async fn exclusion_constraints_stopgap(api: &mut TestApi) -> TestResult {
    let raw_sql = indoc! {r#"
        CREATE EXTENSION btree_gist;

        CREATE TABLE room_reservation (
            room_reservation_id serial PRIMARY KEY,
            room_id integer NOT NULL, -- this could e.g. be a foreign key to a `room` table
            reserved_at timestamptz NOT NULL,
            reserved_until timestamptz NOT NULL,
            canceled boolean DEFAULT false,
            EXCLUDE USING gist (
                room_id WITH =, tstzrange(reserved_at, reserved_until) WITH &&
            ) WHERE (NOT canceled)
        );
    "#};

    api.raw_cmd(raw_sql).await;

    let schema = expect![[r#"
        generator client {
          provider = "prisma-client"
        }

        datasource db {
          provider = "postgresql"
          url      = "env(TEST_DATABASE_URL)"
        }

        /// This table contains exclusion constraints and requires additional setup for migrations. Visit https://pris.ly/d/exclusion-constraints for more info.
        model room_reservation {
          room_reservation_id Int      @id @default(autoincrement())
          room_id             Int
          reserved_at         DateTime @db.Timestamptz(6)
          reserved_until      DateTime @db.Timestamptz(6)
          canceled            Boolean? @default(false)
        }
    "#]];

    api.expect_datamodel(&schema).await;

    // ensure the introspected schema is valid
    psl::parse_schema_without_extensions(schema.data()).unwrap();

    let expectation = expect![[r#"
        *** WARNING ***

        These constraints are not supported by Prisma Client, because Prisma currently does not fully support exclusion constraints. Read more: https://pris.ly/d/exclusion-constraints
          - Model: "room_reservation", constraint: "room_reservation_room_id_tstzrange_excl"
    "#]];

    api.expect_warnings(&expectation).await;

    let input = indoc! { r#"
        /// This table contains exclusion constraints and requires additional setup for migrations. Visit https://pris.ly/d/exclusion-constraints for more info.
        model room_reservation {
          room_reservation_id Int      @id @default(autoincrement())
          room_id             Int
          reserved_at         DateTime @db.Timestamptz(6)
          reserved_until      DateTime @db.Timestamptz(6)
          canceled            Boolean? @default(false)
        }
    "#};

    let expectation = expect![[r#"
        /// This table contains exclusion constraints and requires additional setup for migrations. Visit https://pris.ly/d/exclusion-constraints for more info.
        model room_reservation {
          room_reservation_id Int      @id @default(autoincrement())
          room_id             Int
          reserved_at         DateTime @db.Timestamptz(6)
          reserved_until      DateTime @db.Timestamptz(6)
          canceled            Boolean? @default(false)
        }
    "#]];

    api.expect_re_introspected_datamodel(input, expectation).await;

    Ok(())
}

#[test_connector(tags(Postgres), exclude(CockroachDb))]
async fn exclusion_constraints_without_where_stopgap(api: &mut TestApi) -> TestResult {
    let raw_sql = indoc! {r#"
        CREATE EXTENSION btree_gist;

        CREATE TABLE room_reservation (
            room_reservation_id serial PRIMARY KEY,
            room_id integer NOT NULL, -- this could e.g. be a foreign key to a `room` table
            reserved_at timestamptz NOT NULL,
            reserved_until timestamptz NOT NULL,
            EXCLUDE USING gist (
                room_id WITH =, tstzrange(reserved_at, reserved_until) WITH &&
            )
        );
    "#};

    api.raw_cmd(raw_sql).await;

    let schema = expect![[r#"
        generator client {
          provider = "prisma-client"
        }

        datasource db {
          provider = "postgresql"
          url      = "env(TEST_DATABASE_URL)"
        }

        /// This table contains exclusion constraints and requires additional setup for migrations. Visit https://pris.ly/d/exclusion-constraints for more info.
        model room_reservation {
          room_reservation_id Int      @id @default(autoincrement())
          room_id             Int
          reserved_at         DateTime @db.Timestamptz(6)
          reserved_until      DateTime @db.Timestamptz(6)
        }
    "#]];

    api.expect_datamodel(&schema).await;

    // ensure the introspected schema is valid
    psl::parse_schema_without_extensions(schema.data()).unwrap();

    let expectation = expect![[r#"
        *** WARNING ***

        These constraints are not supported by Prisma Client, because Prisma currently does not fully support exclusion constraints. Read more: https://pris.ly/d/exclusion-constraints
          - Model: "room_reservation", constraint: "room_reservation_room_id_tstzrange_excl"
    "#]];

    api.expect_warnings(&expectation).await;

    let input = indoc! { r#"
        /// This table contains exclusion constraints and requires additional setup for migrations. Visit https://pris.ly/d/exclusion-constraints for more info.
        model room_reservation {
          room_reservation_id Int      @id @default(autoincrement())
          room_id             Int
          reserved_at         DateTime @db.Timestamptz(6)
          reserved_until      DateTime @db.Timestamptz(6)
        }
    "#};

    let expectation = expect![[r#"
        /// This table contains exclusion constraints and requires additional setup for migrations. Visit https://pris.ly/d/exclusion-constraints for more info.
        model room_reservation {
          room_reservation_id Int      @id @default(autoincrement())
          room_id             Int
          reserved_at         DateTime @db.Timestamptz(6)
          reserved_until      DateTime @db.Timestamptz(6)
        }
    "#]];

    api.expect_re_introspected_datamodel(input, expectation).await;

    Ok(())
}

#[test_connector(tags(Postgres), exclude(CockroachDb))]
async fn exclusion_constraints_without_where_and_expressions_stopgap(api: &mut TestApi) -> TestResult {
    let raw_sql = indoc! {r#"
        CREATE EXTENSION btree_gist;

        CREATE TABLE room_reservation (
            room_reservation_id serial PRIMARY KEY,
            room_id integer NOT NULL, -- this could e.g. be a foreign key to a `room` table
            EXCLUDE USING gist (
                room_id WITH =
            )
        );
    "#};

    api.raw_cmd(raw_sql).await;

    let schema = expect![[r#"
        generator client {
          provider = "prisma-client"
        }

        datasource db {
          provider = "postgresql"
          url      = "env(TEST_DATABASE_URL)"
        }

        /// This table contains exclusion constraints and requires additional setup for migrations. Visit https://pris.ly/d/exclusion-constraints for more info.
        model room_reservation {
          room_reservation_id Int @id @default(autoincrement())
          room_id             Int
        }
    "#]];

    api.expect_datamodel(&schema).await;

    // ensure the introspected schema is valid
    psl::parse_schema_without_extensions(schema.data()).unwrap();

    let expectation = expect![[r#"
        *** WARNING ***

        These constraints are not supported by Prisma Client, because Prisma currently does not fully support exclusion constraints. Read more: https://pris.ly/d/exclusion-constraints
          - Model: "room_reservation", constraint: "room_reservation_room_id_excl"
    "#]];

    api.expect_warnings(&expectation).await;

    let input = indoc! { r#"
        /// This table contains exclusion constraints and requires additional setup for migrations. Visit https://pris.ly/d/exclusion-constraints for more info.
        model room_reservation {
          room_reservation_id Int @id @default(autoincrement())
          room_id             Int
        }
    "#};

    let expectation = expect![[r#"
        /// This table contains exclusion constraints and requires additional setup for migrations. Visit https://pris.ly/d/exclusion-constraints for more info.
        model room_reservation {
          room_reservation_id Int @id @default(autoincrement())
          room_id             Int
        }
    "#]];

    api.expect_re_introspected_datamodel(input, expectation).await;

    Ok(())
}

#[test_connector(tags(Postgres), exclude(CockroachDb))]
async fn check_constraints_stopgap(api: &mut TestApi) -> TestResult {
    // https://www.notion.so/prismaio/Indexes-Constraints-Check-constraints-PostgreSQL-cde0bee25f6343d8bbd0f7e84932e808
    let raw_sql = indoc! {r#"
      CREATE TABLE products (
          product_id serial PRIMARY KEY,
          name text,
          price numeric CHECK (price > 0)
      );
    "#};

    api.raw_cmd(raw_sql).await;

    let schema = expect![[r#"
        generator client {
          provider = "prisma-client"
        }

        datasource db {
          provider = "postgresql"
          url      = "env(TEST_DATABASE_URL)"
        }

        /// This table contains check constraints and requires additional setup for migrations. Visit https://pris.ly/d/check-constraints for more info.
        model products {
          product_id Int      @id @default(autoincrement())
          name       String?
          price      Decimal? @db.Decimal
        }
    "#]];

    api.expect_datamodel(&schema).await;

    // ensure the introspected schema is valid
    psl::parse_schema_without_extensions(schema.data()).unwrap();

    let expectation = expect![[r#"
        *** WARNING ***

        These constraints are not supported by Prisma Client, because Prisma currently does not fully support check constraints. Read more: https://pris.ly/d/check-constraints
          - Model: "products", constraint: "products_price_check"
    "#]];

    api.expect_warnings(&expectation).await;

    let input = indoc! { r#"
        /// This table contains check constraints and requires additional setup for migrations. Visit https://pris.ly/d/check-constraints for more info.
        model products {
          product_id Int      @id @default(autoincrement())
          name       String?
          price      Decimal? @db.Decimal
        }
      "#
    };

    let expectation = expect![[r#"
        /// This table contains check constraints and requires additional setup for migrations. Visit https://pris.ly/d/check-constraints for more info.
        model products {
          product_id Int      @id @default(autoincrement())
          name       String?
          price      Decimal? @db.Decimal
        }
    "#]];
    api.expect_re_introspected_datamodel(input, expectation).await;

    Ok(())
}
