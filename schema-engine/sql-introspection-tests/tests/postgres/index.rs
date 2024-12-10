use expect_test::expect;
use sql_introspection_tests::test_api::*;
use test_macros::test_connector;

#[test_connector(tags(Postgres))]
async fn include_indexes_should_not_introspect_included_column(api: &mut TestApi) -> TestResult {
    // https://github.com/prisma/schema-team/issues/399

    let raw_sql = indoc! {r#"
        CREATE TABLE foo (
            id INT PRIMARY KEY,
            val INT NOT NULL,
            val2 INT NOT NULL
        );
        
        CREATE INDEX foo_idx
            ON foo(val) INCLUDE (val2);
       
        CREATE TABLE products (
            id SERIAL PRIMARY KEY,
            category TEXT,
            price NUMERIC(10, 2)
        );
       
        CREATE INDEX products_category_price_idx
            ON products (category, price);
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

        /// This model contains an include index which requires additional setup for migrations. Visit https://pris.ly/d/include-indexes for more info.
        model foo {
          id   Int @id
          val  Int
          val2 Int

          @@index([val], map: "foo_idx")
        }

        model products {
          id       Int      @id @default(autoincrement())
          category String?
          price    Decimal? @db.Decimal(10, 2)

          @@index([category, price])
        }
    "#]];

    api.expect_datamodel(&schema).await;

    let warnings = expect![[r#"
        *** WARNING ***

        These indexes are not supported by Prisma Client, because Prisma currently does not fully support include indexes. Read more: https://pris.ly/d/include-indexes
          - Model: "foo", constraint: "foo_idx"
    "#]];

    api.expect_warnings(&warnings).await;

    Ok(())
}
