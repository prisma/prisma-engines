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

    Ok(())
}
