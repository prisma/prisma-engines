mod brin;
mod constraints;
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
          provider = "prisma-client"
        }

        datasource db {
          provider = "postgresql"
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
        /// This model contains an expression index which requires additional setup for migrations. Visit https://pris.ly/d/expression-indexes for more info.
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
          provider = "prisma-client"
        }

        datasource db {
          provider = "postgresql"
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
          provider = "prisma-client"
        }

        datasource db {
          provider = "postgresql"
        }

        /// This model contains an index with non-default null sort order and requires additional setup for migrations. Visit https://pris.ly/d/default-index-null-ordering for more info.
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
        *** WARNING ***

        These index columns are having a non-default null sort order, which is not yet fully supported. Read more: https://pris.ly/d/non-default-index-null-ordering
          - Index: "idx_a", column: "a"
          - Index: "idx_b", column: "b"
    "#]];

    api.expect_warnings(&expectation).await;

    let input = indoc! {r#"
        /// This model contains an index with non-default null sort order and requires additional setup for migrations. Visit https://pris.ly/d/default-index-null-ordering for more info.
        model foo {
          id Int @id
          a  Int
          b  Int @unique(map: "idx_b", sort: Desc)
          c  Int
          d  Int @unique(map: "idx_d")

          @@index([a], map: "idx_a")
          @@index([c(sort: Desc)], map: "idx_c")
        }
    "#};

    let expectation = expect![[r#"
        /// This model contains an index with non-default null sort order and requires additional setup for migrations. Visit https://pris.ly/d/default-index-null-ordering for more info.
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

    api.expect_re_introspected_datamodel(input, expectation).await;

    Ok(())
}

#[test_connector(tags(Postgres), exclude(CockroachDb))]
async fn deferrable_stopgap(api: &mut TestApi) -> TestResult {
    // https://www.notion.so/prismaio/Indexes-Constraints-Deferred-unique-constraints-PostgreSQL-c302af689bb94a669d645a7aa91765ce

    let schema = indoc! {r#"
        CREATE TABLE a (
            id INT,
            foo INT,
            bar INT
        );

        CREATE TABLE b (
            id INT PRIMARY KEY
        );

        ALTER TABLE a
            ADD CONSTRAINT a_b_fk
            FOREIGN KEY (foo) REFERENCES b(id)
            DEFERRABLE INITIALLY DEFERRED;

        ALTER TABLE a
            ADD CONSTRAINT foo_key
            UNIQUE(foo)
            DEFERRABLE INITIALLY IMMEDIATE;

        ALTER TABLE a
            ADD CONSTRAINT foo_pkey
            PRIMARY KEY (id)
            DEFERRABLE INITIALLY DEFERRED;
    "#};

    api.raw_cmd(schema).await;

    let expectation = expect![[r#"
        generator client {
          provider = "prisma-client"
        }

        datasource db {
          provider = "postgresql"
        }

        /// This model has constraints using non-default deferring rules and requires additional setup for migrations. Visit https://pris.ly/d/constraint-deferring for more info.
        model a {
          id  Int  @id(map: "foo_pkey")
          foo Int? @unique(map: "foo_key")
          bar Int?
          b   b?   @relation(fields: [foo], references: [id], onDelete: NoAction, onUpdate: NoAction, map: "a_b_fk")
        }

        model b {
          id Int @id
          a  a?
        }
    "#]];

    api.expect_datamodel(&expectation).await;

    let expectation = expect![[r#"
        *** WARNING ***

        These primary key, foreign key or unique constraints are using non-default deferring in the database, which is not yet fully supported. Read more: https://pris.ly/d/constraint-deferring
          - Model: "a", constraint: "foo_key"
          - Model: "a", constraint: "foo_pkey"
          - Model: "a", constraint: "a_b_fk"
    "#]];

    api.expect_warnings(&expectation).await;

    let input = indoc! {r#"
        /// This model has constraints using non-default deferring rules and requires additional setup for migrations. Visit https://pris.ly/d/constraint-deferring for more info.
        model a {
          id  Int  @id(map: "foo_pkey")
          foo Int? @unique(map: "foo_key")
          bar Int?
          b   b?   @relation(fields: [foo], references: [id], onDelete: NoAction, onUpdate: NoAction, map: "a_b_fk")
        }

        model b {
          id Int @id
          a  a?
        }
    "#};

    let expectation = expect![[r#"
        /// This model has constraints using non-default deferring rules and requires additional setup for migrations. Visit https://pris.ly/d/constraint-deferring for more info.
        model a {
          id  Int  @id(map: "foo_pkey")
          foo Int? @unique(map: "foo_key")
          bar Int?
          b   b?   @relation(fields: [foo], references: [id], onDelete: NoAction, onUpdate: NoAction, map: "a_b_fk")
        }

        model b {
          id Int @id
          a  a?
        }
    "#]];

    api.expect_re_introspected_datamodel(input, expectation).await;

    Ok(())
}

#[test_connector(tags(Postgres), exclude(CockroachDb), preview_features("views"))]
async fn commenting_stopgap(api: &mut TestApi) -> TestResult {
    // https://www.notion.so/prismaio/Comments-ac89f872098e463183fd668a643f3ab8

    let schema = indoc! {r#"
        CREATE TABLE a (
            id INT PRIMARY KEY,
            val VARCHAR(20)
        );

        CREATE VIEW b AS SELECT val FROM a;

        CREATE TYPE c AS ENUM ('a', 'b');

        COMMENT ON TABLE a IS 'push';
        COMMENT ON COLUMN a.val IS 'meow';
        COMMENT ON VIEW b IS 'purr';
        COMMENT ON TYPE c IS 'hiss';
        COMMENT ON COLUMN b.val IS 'miu';
    "#};

    api.raw_cmd(schema).await;

    let expectation = expect![[r#"
        generator client {
          provider        = "prisma-client"
          previewFeatures = ["views"]
        }

        datasource db {
          provider = "postgresql"
        }

        /// This model or at least one of its fields has comments in the database, and requires an additional setup for migrations: Read more: https://pris.ly/d/database-comments
        model a {
          id  Int     @id
          val String? @db.VarChar(20)
        }

        /// This view or at least one of its fields has comments in the database, and requires an additional setup for migrations: Read more: https://pris.ly/d/database-comments
        view b {
          val String? @db.VarChar(20)
        }

        /// This enum is commented in the database, and requires an additional setup for migrations: Read more: https://pris.ly/d/database-comments
        enum c {
          a
          b
        }
    "#]];

    api.expect_datamodel(&expectation).await;

    let expectation = expect![[r#"
        *** WARNING ***

        These objects have comments defined in the database, which is not yet fully supported. Read more: https://pris.ly/d/database-comments
          - Type: "enum", name: "c"
          - Type: "model", name: "a"
          - Type: "field", name: "a.val"
          - Type: "view", name: "b"
          - Type: "field", name: "b.val"
    "#]];

    api.expect_warnings(&expectation).await;

    let input = indoc! {r#"
        /// This model is commented in the database, and requires an additional setup for migrations: Read more: https://pris.ly/d/database-comments
        model a {
          id  Int     @id
          /// This field is commented in the database, and requires an additional setup for migrations: Read more: https://pris.ly/d/database-comments
          val String? @db.VarChar(20)
        }

        /// This view is commented in the database, and requires an additional setup for migrations: Read more: https://pris.ly/d/database-comments
        view b {
          /// This field is commented in the database, and requires an additional setup for migrations: Read more: https://pris.ly/d/database-comments
          val String? @db.VarChar(20)
        }

        /// This enum is commented in the database, and requires an additional setup for migrations: Read more: https://pris.ly/d/database-comments
        enum c {
          a
          b
        }
    "#};

    let expectation = expect![[r#"
        /// This model is commented in the database, and requires an additional setup for migrations: Read more: https://pris.ly/d/database-comments
        model a {
          id  Int     @id
          /// This field is commented in the database, and requires an additional setup for migrations: Read more: https://pris.ly/d/database-comments
          val String? @db.VarChar(20)
        }

        /// This view is commented in the database, and requires an additional setup for migrations: Read more: https://pris.ly/d/database-comments
        view b {
          /// This field is commented in the database, and requires an additional setup for migrations: Read more: https://pris.ly/d/database-comments
          val String? @db.VarChar(20)
        }

        /// This enum is commented in the database, and requires an additional setup for migrations: Read more: https://pris.ly/d/database-comments
        enum c {
          a
          b
        }
    "#]];

    api.expect_re_introspected_datamodel(input, expectation).await;

    Ok(())
}
