mod brin;
mod extensions;
mod gin;
mod gist;
mod spgist;

use indoc::indoc;
use quaint::prelude::Queryable;
use sql_introspection_tests::test_api::*;
use test_macros::test_connector;

#[test_connector(tags(Postgres), exclude(CockroachDb))]
async fn sequences_should_work(api: &mut TestApi) -> TestResult {
    let setup = r#"
        CREATE SEQUENCE "first_Sequence";
        CREATE SEQUENCE "second_sequence";
        CREATE SEQUENCE "third_Sequence";
 
        CREATE TABLE "Test" (
            id INTEGER PRIMARY KEY,
            serial Serial,
            first BigInt NOT NULL DEFAULT nextval('"first_Sequence"'::regclass),
            second  BigInt Default nextval('"second_sequence"'),
            third  BigInt Not Null Default nextval('"third_Sequence"'::text)
        );
    "#;

    api.raw_cmd(setup).await;

    let expectation = expect![[r#"
        generator client {
          provider = "prisma-client-js"
        }

        datasource db {
          provider = "postgresql"
          url      = "env(TEST_DATABASE_URL)"
        }

        model Test {
          id     Int     @id
          serial Int     @default(autoincrement())
          first  BigInt  @default(autoincrement())
          second BigInt? @default(autoincrement())
          third  BigInt  @default(autoincrement())
        }
    "#]];

    api.expect_datamodel(&expectation).await;

    Ok(())
}

#[test_connector(tags(Postgres), exclude(CockroachDb))]
async fn dbgenerated_type_casts_should_work(api: &mut TestApi) -> TestResult {
    api.barrel()
        .execute(move |migration| {
            migration.create_table("A", move |t| {
                t.inject_custom("id VARCHAR(30) PRIMARY KEY DEFAULT (now())::text");
            });
        })
        .await?;

    let dm = indoc! {r#"
        model A {
          id String @id @default(dbgenerated("(now())::text")) @db.VarChar(30)
        }
    "#};

    let result = api.introspect().await?;
    api.assert_eq_datamodels(dm, &result);

    Ok(())
}

#[test_connector(tags(Postgres), exclude(CockroachDb))]
async fn pg_xml_indexes_are_skipped(api: &mut TestApi) -> TestResult {
    let create_table = format!(
        "CREATE TABLE \"{schema_name}\".xml_test (id SERIAL PRIMARY KEY, data XML)",
        schema_name = api.schema_name()
    );

    let create_primary = format!(
        "CREATE INDEX test_idx ON \"{schema_name}\".xml_test USING BTREE (cast(xpath('/book/title', data) as text[]));",
        schema_name = api.schema_name(),
    );

    api.database().raw_cmd(&create_table).await?;
    api.database().raw_cmd(&create_primary).await?;

    let dm = indoc! {r#"
        model xml_test {
          id   Int @id @default(autoincrement())
          data String? @db.Xml
        }
    "#};

    let result = api.introspect().await?;
    api.assert_eq_datamodels(dm, &result);

    Ok(())
}

#[test_connector(tags(Postgres), exclude(CockroachDb))]
async fn scalar_list_defaults_work(api: &mut TestApi) -> TestResult {
    let schema = r#"
        CREATE TYPE "color" AS ENUM ('RED', 'GREEN', 'BLUE');

        CREATE TABLE "defaults" (
            id TEXT PRIMARY KEY,
            text_empty TEXT[] NOT NULL DEFAULT '{}',
            text TEXT[] NOT NULL DEFAULT '{ ''abc'' }',
            text_c_escape TEXT[] NOT NULL DEFAULT E'{ \'abc\', \'def\' }',
            colors COLOR[] NOT NULL DEFAULT '{ RED, GREEN }',
            int_defaults INT4[] NOT NULL DEFAULT '{ 9, 12999, -4, 0, 1249849 }',
            float_defaults DOUBLE PRECISION[] NOT NULL DEFAULT '{ 0, 9.12, 3.14, 0.1242, 124949.124949 }',
            bool_defaults BOOLEAN[] NOT NULL DEFAULT '{ true, true, true, false }',
            datetime_defaults TIMESTAMPTZ[] NOT NULL DEFAULT '{ ''2022-09-01T08:00Z'',''2021-09-01T08:00Z''}'
        );
    "#;

    api.raw_cmd(schema).await;

    let expectation = expect![[r#"
        generator client {
          provider = "prisma-client-js"
        }

        datasource db {
          provider = "postgresql"
          url      = "env(TEST_DATABASE_URL)"
        }

        model defaults {
          id                String     @id
          text_empty        String[]   @default([])
          text              String[]   @default(["abc"])
          text_c_escape     String[]   @default(["abc", "def"])
          colors            color[]    @default([RED, GREEN])
          int_defaults      Int[]      @default([9, 12999, -4, 0, 1249849])
          float_defaults    Float[]    @default([0, 9.12, 3.14, 0.1242, 124949.124949])
          bool_defaults     Boolean[]  @default([true, true, true, false])
          datetime_defaults DateTime[] @default(dbgenerated("'{\"2022-09-01 08:00:00+00\",\"2021-09-01 08:00:00+00\"}'::timestamp with time zone[]")) @db.Timestamptz
        }

        enum color {
          RED
          GREEN
          BLUE
        }
    "#]];

    api.expect_datamodel(&expectation).await;

    Ok(())
}

#[test_connector(tags(Postgres), exclude(CockroachDb))]
async fn index_sort_order_stopgap(api: &mut TestApi) -> TestResult {
    // https://www.notion.so/prismaio/Index-sort-order-Nulls-first-last-PostgreSQL-cf8265dff0f34dd195732735a4ce9648

    let schema = indoc! {r#"
        CREATE TABLE foo (
            id INT PRIMARY KEY,
            a INT NOT NULL,
            b INT NOT NULL,
            c INT NOT NULL,
            d INT NOT NULL
        );

        CREATE INDEX idx_a ON foo(a ASC NULLS FIRST);
        CREATE UNIQUE INDEX idx_b ON foo(b DESC NULLS LAST);

        -- these two are default orders, no warnings
        CREATE INDEX idx_c ON foo(c DESC NULLS FIRST);
        CREATE UNIQUE INDEX idx_d ON foo(d ASC NULLS LAST);
    "#};

    api.raw_cmd(schema).await;

    let expectation = expect![[r#"
        generator client {
          provider = "prisma-client-js"
        }

        datasource db {
          provider = "postgresql"
          url      = "env(TEST_DATABASE_URL)"
        }

        model foo {
          id Int @id
          a  Int
          b  Int @unique(map: "idx_b", sort: Desc)
          c  Int
          d  Int @unique(map: "idx_d")

          @@index([a], map: "idx_a")
          @@index([c(sort: Desc)], map: "idx_c")
        }
    "#]];

    api.expect_datamodel(&expectation).await;

    let expectation = expect![[r#"
        [
          {
            "code": 29,
            "message": "These index columns are having a non-default null sort order, which is not yet fully supported. Read more: https://pris.ly/d/non-default-index-null-ordering",
            "affected": [
              {
                "indexName": "idx_a",
                "columnName": "a"
              },
              {
                "indexName": "idx_b",
                "columnName": "b"
              }
            ]
          }
        ]"#]];

    api.expect_warnings(&expectation).await;

    Ok(())
}

#[test_connector(tags(Postgres))]
async fn index_sort_order_stopgap_control(api: &mut TestApi) -> TestResult {
    let schema = indoc! {r#"
CREATE TABLE "Post" (
    "id" text NOT NULL,
    "createdAt" timestamp(3) NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updatedAt" timestamp(3) NOT NULL DEFAULT '1970-01-01 00:00:00'::timestamp without time zone,
    "published" boolean NOT NULL DEFAULT false,
    "title" text NOT NULL,
    "content" text,
    "authorId" text,
    "jsonData" jsonb,
    "coinflips" _bool,
    PRIMARY KEY ("id")
);
DROP TABLE IF EXISTS "User" CASCADE;
CREATE TABLE "User" (
    "id" text,
    "email" text NOT NULL,
    "name" text,
    PRIMARY KEY ("id")
);
CREATE UNIQUE INDEX "User.email" ON "User"("email");
ALTER TABLE "Post"
ADD FOREIGN KEY ("authorId") REFERENCES "User"("id") ON DELETE
SET NULL ON UPDATE CASCADE;
INSERT INTO "User" (email, id, name)
VALUES (
        'a@a.de',
        '576eddf9-2434-421f-9a86-58bede16fd95',
        'Alice'
    );
    "#};

    api.raw_cmd(schema).await;

    /*
    this actually returns differnet things for Postgres and Cockroach, this here is the Postgres one
    let expectation = expect![[r#"
        generator client {
          provider = "prisma-client-js"
        }

        datasource db {
          provider = "postgresql"
          url      = "env(TEST_DATABASE_URL)"
        }

        model Post {
          id        String    @id
          createdAt DateTime  @default(now())
          updatedAt DateTime  @default(dbgenerated("'1970-01-01 00:00:00'::timestamp without time zone"))
          published Boolean   @default(false)
          title     String
          content   String?
          authorId  String?
          jsonData  Json?
          coinflips Boolean[]
          User      User?     @relation(fields: [authorId], references: [id])
        }

        model User {
          id    String  @id
          email String  @unique(map: "User.email")
          name  String?
          Post  Post[]
        }
    "#]];

    api.expect_datamodel(&expectation).await;
    */

    let expectation = expect![[r#"
        "#]];

    api.expect_warnings(&expectation).await;

    Ok(())
}
