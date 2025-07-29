use expect_test::expect;
use indoc::indoc;
use quaint::prelude::Queryable;
use sql_introspection_tests::test_api::*;
use test_macros::test_connector;

#[test_connector(tags(Postgres), exclude(CockroachDb))]
async fn partial_index_basic(api: &mut TestApi) -> TestResult {
    let schema_name = api.schema_name();
    let create_table = format!(
        "CREATE TABLE \"{schema_name}\".\"User\" (id SERIAL PRIMARY KEY, email VARCHAR(255), active BOOLEAN NOT NULL DEFAULT false)"
    );
    let create_idx = format!(
        "CREATE INDEX \"User_email_idx\" ON \"{schema_name}\".\"User\" (email) WHERE email IS NOT NULL"
    );

    api.database().raw_cmd(&create_table).await?;
    api.database().raw_cmd(&create_idx).await?;

    let expected = expect![[r#"
        generator client {
          provider        = "prisma-client-js"
          previewFeatures = ["partialIndexes"]
        }

        datasource db {
          provider = "postgresql"
          url      = "env(TEST_DATABASE_URL)"
        }

        model User {
          id     Int     @id @default(autoincrement())
          email  String? @db.VarChar(255)
          active Boolean @default(false)

          @@index([email], where: "(email IS NOT NULL)")
        }
    "#]];

    let result = api.introspect().await?;
    expected.assert_eq(&result);

    Ok(())
}

#[test_connector(tags(Postgres), exclude(CockroachDb))]
async fn partial_unique_index(api: &mut TestApi) -> TestResult {
    let schema_name = api.schema_name();
    let create_table = format!(
        "CREATE TABLE \"{schema_name}\".\"Post\" (id SERIAL PRIMARY KEY, slug VARCHAR(255), published BOOLEAN NOT NULL DEFAULT false)"
    );
    let create_idx = format!(
        "CREATE UNIQUE INDEX \"Post_slug_key\" ON \"{schema_name}\".\"Post\" (slug) WHERE published = true"
    );

    api.database().raw_cmd(&create_table).await?;
    api.database().raw_cmd(&create_idx).await?;

    let expected = expect![[r#"
        model Post {
          id        Int     @id @default(autoincrement())
          slug      String? @unique @db.VarChar(255)
          published Boolean @default(false)
        }
    "#]];

    let result = api.introspect_dml().await?;
    expected.assert_eq(&result);

    Ok(())
}

#[test_connector(tags(Postgres), exclude(CockroachDb))]
async fn compound_partial_index(api: &mut TestApi) -> TestResult {
    let schema_name = api.schema_name();
    let create_table = format!(
        "CREATE TABLE \"{schema_name}\".\"Order\" (id SERIAL PRIMARY KEY, user_id INT NOT NULL, status VARCHAR(20) NOT NULL, created_at TIMESTAMP)"
    );
    let create_idx = format!(
        "CREATE INDEX \"Order_user_status_idx\" ON \"{schema_name}\".\"Order\" (user_id, status) WHERE status IN ('pending', 'processing')"
    );

    api.database().raw_cmd(&create_table).await?;
    api.database().raw_cmd(&create_idx).await?;

    let expected = expect![[r#"
        model Order {
          id         Int       @id @default(autoincrement())
          user_id    Int
          status     String    @db.VarChar(20)
          created_at DateTime? @db.Timestamp(6)

          @@index([user_id, status], map: "Order_user_status_idx", where: "((status)::text = ANY ((ARRAY['pending'::character varying, 'processing'::character varying])::text[]))")
        }
    "#]];

    let result = api.introspect_dml().await?;
    expected.assert_eq(&result);

    Ok(())
}

#[test_connector(tags(Postgres), exclude(CockroachDb))]
async fn multiple_partial_indexes_same_table(api: &mut TestApi) -> TestResult {
    let schema_name = api.schema_name();
    let create_table = format!(
        "CREATE TABLE \"{schema_name}\".\"Product\" (id SERIAL PRIMARY KEY, name VARCHAR(255), price DECIMAL(10,2), active BOOLEAN DEFAULT true, category_id INT)"
    );
    let create_idx1 = format!(
        "CREATE INDEX \"Product_active_idx\" ON \"{schema_name}\".\"Product\" (name) WHERE active = true"
    );
    let create_idx2 = format!(
        "CREATE INDEX \"Product_expensive_idx\" ON \"{schema_name}\".\"Product\" (price) WHERE price > 100.00"
    );

    api.database().raw_cmd(&create_table).await?;
    api.database().raw_cmd(&create_idx1).await?;
    api.database().raw_cmd(&create_idx2).await?;

    let expected = expect![[r#"
        model Product {
          id          Int      @id @default(autoincrement())
          name        String?  @db.VarChar(255)
          price       Decimal? @db.Decimal(10, 2)
          active      Boolean? @default(true)
          category_id Int?

          @@index([name], map: "Product_active_idx", where: "(active = true)")
          @@index([price], map: "Product_expensive_idx", where: "(price > 100.00)")
        }
    "#]];

    let result = api.introspect_dml().await?;
    expected.assert_eq(&result);

    Ok(())
}

#[test_connector(tags(Postgres), exclude(CockroachDb))]
async fn partial_index_with_gin_type(api: &mut TestApi) -> TestResult {
    let schema_name = api.schema_name();
    let create_table = format!(
        "CREATE TABLE \"{schema_name}\".\"Document\" (id SERIAL PRIMARY KEY, tags TEXT[], published BOOLEAN DEFAULT false)"
    );
    let create_idx = format!(
        "CREATE INDEX \"Document_tags_idx\" ON \"{schema_name}\".\"Document\" USING GIN (tags) WHERE published = true"
    );

    api.database().raw_cmd(&create_table).await?;
    api.database().raw_cmd(&create_idx).await?;

    let expected = expect![[r#"
        model Document {
          id        Int      @id @default(autoincrement())
          tags      String[]
          published Boolean? @default(false)

          @@index([tags], type: Gin, where: "(published = true)")
        }
    "#]];

    let result = api.introspect_dml().await?;
    expected.assert_eq(&result);

    Ok(())
}

#[test_connector(tags(Postgres), exclude(CockroachDb), preview_features("partialIndexes"))]
async fn re_introspect_partial_index(api: &mut TestApi) -> TestResult {
    let schema_name = api.schema_name();
    let create_table = format!(
        "CREATE TABLE \"{schema_name}\".\"Task\" (id SERIAL PRIMARY KEY, title VARCHAR(255), completed BOOLEAN DEFAULT false)"
    );
    let create_idx = format!(
        "CREATE INDEX \"Task_title_idx\" ON \"{schema_name}\".\"Task\" (title) WHERE completed = false"
    );

    api.database().raw_cmd(&create_table).await?;
    api.database().raw_cmd(&create_idx).await?;

    let expected = expect![[r#"
        generator client {
          provider        = "prisma-client-js"
          previewFeatures = ["partialIndexes"]
        }

        datasource db {
          provider = "postgresql"
          url      = "env(TEST_DATABASE_URL)"
        }

        model Task {
          id        Int      @id @default(autoincrement())
          title     String?  @db.VarChar(255)
          completed Boolean? @default(false)

          @@index([title], where: "(completed = false)")
        }
    "#]];

    let result = api.introspect().await?;
    expected.assert_eq(&result);

    // Test re-introspection to ensure schema stability
    let input = indoc! {r#"
        model Task {
          id        Int      @id @default(autoincrement())
          title     String?  @db.VarChar(255)
          completed Boolean? @default(false)

          @@index([title], where: "(completed = false)")
        }
    "#};

    let expectation = expect![[r#"
        model Task {
          id        Int      @id @default(autoincrement())
          title     String?  @db.VarChar(255)
          completed Boolean? @default(false)

          @@index([title], where: "(completed = false)")
        }
    "#]];

    api.expect_re_introspected_datamodel(input, expectation).await;

    Ok(())
}
