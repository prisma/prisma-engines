//! https://www.notion.so/prismaio/PostgreSQL-Exclusion-Constraints-fb2ecc44f773463f908d3d0e2d737271

use indoc::indoc;
use sql_introspection_tests::test_api::*;
use test_macros::test_connector;

#[test_connector(tags(Postgres))]
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
          provider = "prisma-client-js"
        }

        datasource db {
          provider = "postgresql"
          url      = "env(TEST_DATABASE_URL)"
        }

        /// This table contains check constraints and requires additional setup for migrations. Visit https://pris.ly/d/postgres-check-constraints for more info.
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
        [
          {
            "code": 33,
            "message": "These constraints are not supported by the Prisma Client, because Prisma currently does not fully support check constraints. Read more: https://pris.ly/d/postgres-check-constraints",
            "affected": [
              {
                "model": "tokens",
                "constraint": "tokens_token_scope_check"
              }
            ]
          }
        ]"#]];

    api.expect_warnings(&expectation).await;

    Ok(())
}

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
        [
          {
            "code": 33,
            "message": "These constraints are not supported by the Prisma Client, because Prisma currently does not fully support check constraints. Read more: https://pris.ly/d/postgres-check-constraints",
            "affected": [
              {
                "model": "tokens",
                "constraint": "tokens_token_scope_check"
              }
            ]
          }
        ]"#]];

    api.expect_warnings(&expectation).await;

    Ok(())
}

#[test_connector(tags(Postgres, CockroachDb))]
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
          provider = "prisma-client-js"
        }

        datasource db {
          provider = "postgresql"
          url      = "env(TEST_DATABASE_URL)"
        }

        /// This table contains check constraints and requires additional setup for migrations. Visit https://pris.ly/d/postgres-check-constraints for more info.
        /// This table contains exclusion constraints and requires additional setup for migrations. Visit https://pris.ly/d/postgres-exclusion-constraints for more info.
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
    psl::parse_schema(schema.data()).unwrap();

    let expectation = expect![[r#"
        [
          {
            "code": 33,
            "message": "These constraints are not supported by the Prisma Client, because Prisma currently does not fully support check constraints. Read more: https://pris.ly/d/postgres-check-constraints",
            "affected": [
              {
                "model": "room_reservation",
                "constraint": "room_reservation_price_check"
              }
            ]
          },
          {
            "code": 34,
            "message": "These constraints are not supported by the Prisma Client, because Prisma currently does not fully support exclusion constraints. Read more: https://pris.ly/d/postgres-exclusion-constraints",
            "affected": [
              {
                "model": "room_reservation",
                "constraint": "room_reservation_room_id_tstzrange_excl"
              }
            ]
          }
        ]"#]];

    api.expect_warnings(&expectation).await;

    let input = indoc! { r#"
        /// This table contains check constraints and requires additional setup for migrations. Visit https://pris.ly/d/postgres-check-constraints for more info.
        /// This table contains exclusion constraints and requires additional setup for migrations. Visit https://pris.ly/d/postgres-exclusion-constraints for more info.
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
        /// This table contains check constraints and requires additional setup for migrations. Visit https://pris.ly/d/postgres-check-constraints for more info.
        /// This table contains exclusion constraints and requires additional setup for migrations. Visit https://pris.ly/d/postgres-exclusion-constraints for more info.
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
          provider = "prisma-client-js"
        }

        datasource db {
          provider = "postgresql"
          url      = "env(TEST_DATABASE_URL)"
        }

        /// This table contains exclusion constraints and requires additional setup for migrations. Visit https://pris.ly/d/postgres-exclusion-constraints for more info.
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
    psl::parse_schema(schema.data()).unwrap();

    let expectation = expect![[r#"
        [
          {
            "code": 34,
            "message": "These constraints are not supported by the Prisma Client, because Prisma currently does not fully support exclusion constraints. Read more: https://pris.ly/d/postgres-exclusion-constraints",
            "affected": [
              {
                "model": "room_reservation",
                "constraint": "room_reservation_room_id_tstzrange_excl"
              }
            ]
          }
        ]"#]];

    api.expect_warnings(&expectation).await;

    let input = indoc! { r#"
        /// This table contains exclusion constraints and requires additional setup for migrations. Visit https://pris.ly/d/postgres-exclusion-constraints for more info.
        model room_reservation {
          room_reservation_id Int      @id @default(autoincrement())
          room_id             Int
          reserved_at         DateTime @db.Timestamptz(6)
          reserved_until      DateTime @db.Timestamptz(6)
          canceled            Boolean? @default(false)
        }
    "#};

    let expectation = expect![[r#"
        /// This table contains exclusion constraints and requires additional setup for migrations. Visit https://pris.ly/d/postgres-exclusion-constraints for more info.
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
          provider = "prisma-client-js"
        }

        datasource db {
          provider = "postgresql"
          url      = "env(TEST_DATABASE_URL)"
        }

        /// This table contains exclusion constraints and requires additional setup for migrations. Visit https://pris.ly/d/postgres-exclusion-constraints for more info.
        model room_reservation {
          room_reservation_id Int      @id @default(autoincrement())
          room_id             Int
          reserved_at         DateTime @db.Timestamptz(6)
          reserved_until      DateTime @db.Timestamptz(6)
        }
    "#]];

    api.expect_datamodel(&schema).await;

    // ensure the introspected schema is valid
    psl::parse_schema(schema.data()).unwrap();

    let expectation = expect![[r#"
        [
          {
            "code": 34,
            "message": "These constraints are not supported by the Prisma Client, because Prisma currently does not fully support exclusion constraints. Read more: https://pris.ly/d/postgres-exclusion-constraints",
            "affected": [
              {
                "model": "room_reservation",
                "constraint": "room_reservation_room_id_tstzrange_excl"
              }
            ]
          }
        ]"#]];

    api.expect_warnings(&expectation).await;

    let input = indoc! { r#"
        /// This table contains exclusion constraints and requires additional setup for migrations. Visit https://pris.ly/d/postgres-exclusion-constraints for more info.
        model room_reservation {
          room_reservation_id Int      @id @default(autoincrement())
          room_id             Int
          reserved_at         DateTime @db.Timestamptz(6)
          reserved_until      DateTime @db.Timestamptz(6)
        }
    "#};

    let expectation = expect![[r#"
        /// This table contains exclusion constraints and requires additional setup for migrations. Visit https://pris.ly/d/postgres-exclusion-constraints for more info.
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
          provider = "prisma-client-js"
        }

        datasource db {
          provider = "postgresql"
          url      = "env(TEST_DATABASE_URL)"
        }

        /// This table contains exclusion constraints and requires additional setup for migrations. Visit https://pris.ly/d/postgres-exclusion-constraints for more info.
        model room_reservation {
          room_reservation_id Int @id @default(autoincrement())
          room_id             Int
        }
    "#]];

    api.expect_datamodel(&schema).await;

    // ensure the introspected schema is valid
    psl::parse_schema(schema.data()).unwrap();

    let expectation = expect![[r#"
        [
          {
            "code": 34,
            "message": "These constraints are not supported by the Prisma Client, because Prisma currently does not fully support exclusion constraints. Read more: https://pris.ly/d/postgres-exclusion-constraints",
            "affected": [
              {
                "model": "room_reservation",
                "constraint": "room_reservation_room_id_excl"
              }
            ]
          }
        ]"#]];

    api.expect_warnings(&expectation).await;

    let input = indoc! { r#"
        /// This table contains exclusion constraints and requires additional setup for migrations. Visit https://pris.ly/d/postgres-exclusion-constraints for more info.
        model room_reservation {
          room_reservation_id Int @id @default(autoincrement())
          room_id             Int
        }
    "#};

    let expectation = expect![[r#"
        /// This table contains exclusion constraints and requires additional setup for migrations. Visit https://pris.ly/d/postgres-exclusion-constraints for more info.
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
            provider = "prisma-client-js"
          }

          datasource db {
            provider = "postgresql"
            url      = "env(TEST_DATABASE_URL)"
          }

          /// This table contains check constraints and requires additional setup for migrations. Visit https://pris.ly/d/postgres-check-constraints for more info.
          model products {
            product_id Int      @id @default(autoincrement())
            name       String?
            price      Decimal? @db.Decimal
          }
      "#]];

    api.expect_datamodel(&schema).await;

    // ensure the introspected schema is valid
    psl::parse_schema(schema.data()).unwrap();

    let expectation = expect![[r#"
          [
            {
              "code": 33,
              "message": "These constraints are not supported by the Prisma Client, because Prisma currently does not fully support check constraints. Read more: https://pris.ly/d/postgres-check-constraints",
              "affected": [
                {
                  "model": "products",
                  "constraint": "products_price_check"
                }
              ]
            }
          ]"#]];

    api.expect_warnings(&expectation).await;

    let input = indoc! { r#"
        /// This table contains check constraints and requires additional setup for migrations. Visit https://pris.ly/d/postgres-check-constraints for more info.
        model products {
          product_id Int      @id @default(autoincrement())
          name       String?
          price      Decimal? @db.Decimal
        }
      "#
    };

    let expectation = expect![[r#"
          /// This table contains check constraints and requires additional setup for migrations. Visit https://pris.ly/d/postgres-check-constraints for more info.
          model products {
            product_id Int      @id @default(autoincrement())
            name       String?
            price      Decimal? @db.Decimal
          }
      "#]];
    api.expect_re_introspected_datamodel(input, expectation).await;

    Ok(())
}
