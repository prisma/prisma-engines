use indoc::indoc;
use sql_introspection_tests::test_api::*;

#[test_connector(tags(Postgres), exclude(CockroachDb))]
async fn preview_feature_is_required(api: &mut TestApi) -> TestResult {
    let setup = indoc! {r#"
        CREATE TABLE "User" (
            id SERIAL PRIMARY KEY,
            first_name VARCHAR(255) NOT NULL,
            last_name VARCHAR(255) NULL
        );

        CREATE VIEW "Schwuser" AS
            SELECT id, first_name, last_name FROM "User";
    "#};

    api.raw_cmd(setup).await;

    let expected = expect![[r#"
        generator client {
          provider = "prisma-client-js"
        }

        datasource db {
          provider = "postgresql"
          url      = "env(TEST_DATABASE_URL)"
        }

        model User {
          id         Int     @id @default(autoincrement())
          first_name String  @db.VarChar(255)
          last_name  String? @db.VarChar(255)
        }
    "#]];

    api.expect_datamodel(&expected).await;

    Ok(())
}

// See next test after this one for PostgreSQL 16
#[test_connector(tags(Postgres), exclude(Postgres16, CockroachDb), preview_features("views"))]
async fn simple_view_from_one_table(api: &mut TestApi) -> TestResult {
    let setup = indoc! {r#"
        CREATE TABLE "User" (
            id SERIAL PRIMARY KEY,
            first_name VARCHAR(255) NOT NULL,
            last_name VARCHAR(255) NULL
        );

        CREATE VIEW "Schwuser" AS
            SELECT id, first_name, last_name FROM "User";
    "#};

    api.raw_cmd(setup).await;

    let expected = expect![[r#"
        generator client {
          provider        = "prisma-client-js"
          previewFeatures = ["views"]
        }

        datasource db {
          provider = "postgresql"
          url      = "env(TEST_DATABASE_URL)"
        }

        model User {
          id         Int     @id @default(autoincrement())
          first_name String  @db.VarChar(255)
          last_name  String? @db.VarChar(255)
        }

        view Schwuser {
          id         Int?
          first_name String? @db.VarChar(255)
          last_name  String? @db.VarChar(255)
        }
    "#]];

    api.expect_datamodel(&expected).await;

    let expected = expect![[r#"
        SELECT
          "User".id,
          "User".first_name,
          "User".last_name
        FROM
          "User";"#]];

    api.expect_view_definition("Schwuser", &expected).await;

    api.expect_no_warnings().await;

    Ok(())
}

// PostgreSQL 16 only
// the expect_view_definition is slightly different than for previous versions
#[test_connector(tags(Postgres16), preview_features("views"))]
async fn simple_view_from_one_table_postgres16(api: &mut TestApi) -> TestResult {
    let setup = indoc! {r#"
        CREATE TABLE "User" (
            id SERIAL PRIMARY KEY,
            first_name VARCHAR(255) NOT NULL,
            last_name VARCHAR(255) NULL
        );

        CREATE VIEW "Schwuser" AS
            SELECT id, first_name, last_name FROM "User";
    "#};

    api.raw_cmd(setup).await;

    let expected = expect![[r#"
        generator client {
          provider        = "prisma-client-js"
          previewFeatures = ["views"]
        }

        datasource db {
          provider = "postgresql"
          url      = "env(TEST_DATABASE_URL)"
        }

        model User {
          id         Int     @id @default(autoincrement())
          first_name String  @db.VarChar(255)
          last_name  String? @db.VarChar(255)
        }

        view Schwuser {
          id         Int?
          first_name String? @db.VarChar(255)
          last_name  String? @db.VarChar(255)
        }
    "#]];

    api.expect_datamodel(&expected).await;

    // For Postgres <16 it looks like this:
    // SELECT
    //   "User".id,
    //   "User".first_name,
    //   "User".last_name
    // FROM
    //   "User";
    let expected = expect![[r#"
        SELECT
          id,
          first_name,
          last_name
        FROM
          "User";"#]];

    api.expect_view_definition("Schwuser", &expected).await;

    api.expect_no_warnings().await;

    Ok(())
}

#[test_connector(tags(Postgres), exclude(CockroachDb), preview_features("views"))]
async fn simple_view_from_two_tables(api: &mut TestApi) -> TestResult {
    let setup = indoc! {r#"
        CREATE TABLE "User" (
            id SERIAL PRIMARY KEY,
            first_name VARCHAR(255) NOT NULL,
            last_name VARCHAR(255) NULL
        );

        CREATE TABLE "Profile" (
            user_id INT PRIMARY KEY,
            introduction TEXT,
            CONSTRAINT Profile_User_fkey FOREIGN KEY (user_id) REFERENCES "User"(id)
        );

        CREATE VIEW "Schwuser" AS
            SELECT
                u.id,
                CONCAT(u.first_name, ' ', u.last_name) AS name,
                p.introduction
            FROM "User" u
            INNER JOIN "Profile" p ON u.id = p.user_id;
    "#};

    api.raw_cmd(setup).await;

    let expected = expect![[r#"
        generator client {
          provider        = "prisma-client-js"
          previewFeatures = ["views"]
        }

        datasource db {
          provider = "postgresql"
          url      = "env(TEST_DATABASE_URL)"
        }

        model Profile {
          user_id      Int     @id
          introduction String?
          User         User    @relation(fields: [user_id], references: [id], onDelete: NoAction, onUpdate: NoAction, map: "profile_user_fkey")
        }

        model User {
          id         Int      @id @default(autoincrement())
          first_name String   @db.VarChar(255)
          last_name  String?  @db.VarChar(255)
          Profile    Profile?
        }

        view Schwuser {
          id           Int?
          name         String?
          introduction String?
        }
    "#]];

    api.expect_datamodel(&expected).await;

    let expected = expect![[r#"
        SELECT
          u.id,
          concat(u.first_name, ' ', u.last_name) AS name,
          p.introduction
        FROM
          (
            "User" u
            JOIN "Profile" p ON ((u.id = p.user_id))
          );"#]];

    api.expect_view_definition("Schwuser", &expected).await;

    Ok(())
}

#[test_connector(tags(Postgres), exclude(CockroachDb), preview_features("views"))]
async fn re_intro_keeps_column_arity_and_unique(api: &mut TestApi) -> TestResult {
    let setup = indoc! {r#"
        CREATE TABLE "User" (
            id SERIAL PRIMARY KEY,
            first_name VARCHAR(255) NOT NULL,
            last_name VARCHAR(255) NULL
        );

        CREATE VIEW "Schwuser" AS
            SELECT id, first_name, last_name FROM "User";
    "#};

    api.raw_cmd(setup).await;

    let input = indoc! {r#"
        model User {
          id         Int     @id @default(autoincrement())
          first_name String  @db.VarChar(255)
          last_name  String? @db.VarChar(255)
        }

        view Schwuser {
          id         Int
          first_name String  @db.VarChar(255)
          last_name  String? @db.VarChar(255)
        }
    "#};

    let expected = expect![[r#"
        model User {
          id         Int     @id @default(autoincrement())
          first_name String  @db.VarChar(255)
          last_name  String? @db.VarChar(255)
        }

        view Schwuser {
          id         Int
          first_name String  @db.VarChar(255)
          last_name  String? @db.VarChar(255)
        }
    "#]];

    api.expect_re_introspected_datamodel(input, expected).await;

    Ok(())
}

#[test_connector(tags(Postgres), exclude(CockroachDb), preview_features("views"))]
async fn re_intro_does_not_keep_column_arity_if_list(api: &mut TestApi) -> TestResult {
    let setup = indoc! {r#"
        CREATE TABLE "User" (
            id SERIAL PRIMARY KEY,
            val INT [] NOT NULL
        );

        CREATE VIEW "Schwuser" AS
            SELECT id, val FROM "User";
    "#};

    api.raw_cmd(setup).await;

    let input = indoc! {r#"
        model User {
          id  Int   @id @default(autoincrement())
          val Int[]
        }

        view Schwuser {
          id  Int
          val Int
        }
    "#};

    let expected = expect![[r#"
        model User {
          id  Int   @id @default(autoincrement())
          val Int[]
        }

        view Schwuser {
          id  Int
          val Int[]
        }
    "#]];

    api.expect_re_introspected_datamodel(input, expected).await;

    Ok(())
}

#[test_connector(tags(Postgres), exclude(CockroachDb), preview_features("views"))]
async fn re_intro_keeps_comments(api: &mut TestApi) -> TestResult {
    let setup = indoc! {r#"
        CREATE VIEW "A" AS SELECT 1 AS id;
    "#};

    api.raw_cmd(setup).await;

    let input = indoc! {r#"
        /// I'm a view doc
        view A {
          /// I'm a field doc
          id Int
        }
    "#};

    let expected = expect![[r#"
        /// I'm a view doc
        view A {
          /// I'm a field doc
          id Int
        }
    "#]];

    api.expect_re_introspected_datamodel(input, expected).await;

    Ok(())
}

#[test_connector(tags(Postgres), exclude(CockroachDb), preview_features("views"))]
async fn re_intro_ignores_the_ignored(api: &mut TestApi) -> TestResult {
    let setup = indoc! {r#"
        CREATE VIEW "A" AS SELECT 1 AS id;
    "#};

    api.raw_cmd(setup).await;

    let input = indoc! {r#"
        view A {
          id Int

          @@ignore
        }
    "#};

    let expected = expect![[r#"
        view A {
          id Int

          @@ignore
        }
    "#]];

    api.expect_re_introspected_datamodel(input, expected).await;

    Ok(())
}

#[test_connector(tags(Postgres), exclude(CockroachDb), preview_features("views"))]
async fn reserved_name_gets_mapped(api: &mut TestApi) -> TestResult {
    let setup = indoc! {r#"
        CREATE VIEW "if" AS SELECT 1 AS id;
    "#};

    api.raw_cmd(setup).await;

    let expected = expect![[r#"
        generator client {
          provider        = "prisma-client-js"
          previewFeatures = ["views"]
        }

        datasource db {
          provider = "postgresql"
          url      = "env(TEST_DATABASE_URL)"
        }

        /// This view has been renamed to 'Renamedif' during introspection, because the original name 'if' is reserved.
        view Renamedif {
          id Int?

          @@map("if")
        }
    "#]];

    api.expect_datamodel(&expected).await;

    Ok(())
}

#[test_connector(tags(Postgres), exclude(CockroachDb), preview_features("views"))]
async fn unsupported_types_trigger_a_warning(api: &mut TestApi) -> TestResult {
    let setup = indoc! {r#"
        CREATE VIEW "A" AS SELECT 1 AS id, to_tsvector('english', 'meow') as vector;
    "#};

    api.raw_cmd(setup).await;

    let input = indoc! {r#"
        view A {
          id Int
        }
    "#};

    let expected = expect![[r#"
        view A {
          id     Int
          vector Unsupported("tsvector")?
        }
    "#]];

    api.expect_re_introspected_datamodel(input, expected).await;

    let expected = expect![[r#"
        *** WARNING ***

        These fields are not supported by Prisma Client, because Prisma currently does not support their types:
          - View: "A", field: "vector", original data type: "tsvector"
    "#]];

    api.expect_warnings(&expected).await;

    Ok(())
}

#[test_connector(tags(Postgres), exclude(CockroachDb), preview_features("views"))]
async fn re_intro_keeps_the_map(api: &mut TestApi) -> TestResult {
    let setup = indoc! {r#"
        CREATE VIEW "A" AS SELECT 1 AS id;
    "#};

    api.raw_cmd(setup).await;

    let input = indoc! {r#"
        view B {
          id Int

          @@map("A")
        }
    "#};

    let expected = expect![[r#"
        view B {
          id Int

          @@map("A")
        }
    "#]];

    api.expect_re_introspected_datamodel(input, expected).await;

    let expected = expect![[r#"
        *** WARNING ***

        These views were enriched with `@@map` information taken from the previous Prisma schema:
          - "B"
    "#]];

    api.expect_re_introspect_warnings(input, expected).await;

    Ok(())
}

#[test_connector(tags(Postgres), exclude(CockroachDb), preview_features("views"))]
async fn re_intro_keeps_the_field_map(api: &mut TestApi) -> TestResult {
    let setup = indoc! {r#"
        CREATE VIEW "A" AS SELECT 1 AS id;
    "#};

    api.raw_cmd(setup).await;

    let input = indoc! {r#"
        view A {
          meow Int @map("id")
        }
    "#};

    let expected = expect![[r#"
        view A {
          meow Int @map("id")
        }
    "#]];

    api.expect_re_introspected_datamodel(input, expected).await;

    let expected = expect![[r#"
        *** WARNING ***

        These fields were enriched with `@map` information taken from the previous Prisma schema:
          - View: "A", field: "meow"
    "#]];

    api.expect_re_introspect_warnings(input, expected).await;

    Ok(())
}

#[test_connector(
    tags(Postgres),
    exclude(CockroachDb),
    preview_features("views"),
    namespaces("public")
)]
async fn schema_is_introspected(api: &mut TestApi) -> TestResult {
    let setup = indoc! {r#"
        CREATE VIEW public."A" AS SELECT 1 AS id;
    "#};

    api.raw_cmd(setup).await;

    let expected = expect![[r#"
        generator client {
          provider        = "prisma-client-js"
          previewFeatures = ["views"]
        }

        datasource db {
          provider = "postgresql"
          url      = "env(TEST_DATABASE_URL)"
          schemas  = ["public"]
        }

        view A {
          id Int?

          @@schema("public")
        }
    "#]];

    api.expect_datamodel(&expected).await;

    let expected = expect![[r#"
        SELECT
          1 AS id;"#]];

    api.expect_view_definition_in_schema("public", "A", &expected).await;

    Ok(())
}

#[test_connector(tags(Postgres), exclude(CockroachDb), preview_features("views"))]
async fn defaults_are_introspected(api: &mut TestApi) -> TestResult {
    let setup = indoc! {r#"
        CREATE VIEW "A" AS SELECT 1 AS id;
        ALTER VIEW "A" ALTER COLUMN id SET DEFAULT 3;
    "#};

    api.raw_cmd(setup).await;

    let expected = expect![[r#"
        generator client {
          provider        = "prisma-client-js"
          previewFeatures = ["views"]
        }

        datasource db {
          provider = "postgresql"
          url      = "env(TEST_DATABASE_URL)"
        }

        view A {
          id Int? @default(3)
        }
    "#]];

    api.expect_datamodel(&expected).await;

    Ok(())
}

#[test_connector(tags(Postgres), exclude(CockroachDb), preview_features("views"))]
async fn invalid_field_names_trigger_warnings(api: &mut TestApi) -> TestResult {
    let setup = indoc! {r#"
        CREATE VIEW "A" AS SELECT 2 AS foo, 1 AS "1";
    "#};

    api.raw_cmd(setup).await;

    let expected = expect![[r#"
        generator client {
          provider        = "prisma-client-js"
          previewFeatures = ["views"]
        }

        datasource db {
          provider = "postgresql"
          url      = "env(TEST_DATABASE_URL)"
        }

        view A {
          foo Int?
          /// This field was commented out because of an invalid name. Please provide a valid one that matches [a-zA-Z][a-zA-Z0-9_]*
          // 1 Int? @map("1")
        }
    "#]];

    api.expect_datamodel(&expected).await;

    let expected = expect![[r#"
        *** WARNING ***

        These fields were commented out because their names are currently not supported by Prisma. Please provide valid ones that match [a-zA-Z][a-zA-Z0-9_]* using the `@map` attribute:
          - View: "A", field(s): ["1"]
    "#]];

    api.expect_warnings(&expected).await;

    Ok(())
}

#[test_connector(
    tags(Postgres),
    exclude(CockroachDb),
    preview_features("views"),
    namespaces("public", "private")
)]
async fn dupes_are_renamed(api: &mut TestApi) -> TestResult {
    let setup = indoc! {r#"
        CREATE SCHEMA IF NOT EXISTS "private";
        CREATE VIEW public."A" AS SELECT 1 AS id;
        CREATE TABLE private."A" (id INT PRIMARY KEY);
    "#};

    api.raw_cmd(setup).await;

    let expected = expect![[r#"
        generator client {
          provider        = "prisma-client-js"
          previewFeatures = ["views"]
        }

        datasource db {
          provider = "postgresql"
          url      = "env(TEST_DATABASE_URL)"
          schemas  = ["private", "public"]
        }

        model private_A {
          id Int @id

          @@map("A")
          @@schema("private")
        }

        view public_A {
          id Int?

          @@map("A")
          @@schema("public")
        }
    "#]];

    api.expect_datamodel(&expected).await;

    let expected = expect![[r#"
        *** WARNING ***

        These items were renamed due to their names being duplicates in the Prisma schema:
          - Type: "model", name: "private_A"
          - Type: "view", name: "public_A"
    "#]];

    api.expect_warnings(&expected).await;

    Ok(())
}

#[test_connector(tags(Postgres), exclude(CockroachDb), namespaces("public", "private"))]
async fn dupe_views_are_not_considered_without_preview_feature(api: &mut TestApi) -> TestResult {
    let setup = indoc! {r#"
        CREATE SCHEMA IF NOT EXISTS "private";
        CREATE VIEW public."A" AS SELECT 1 AS id;
        CREATE TABLE private."A" (id INT PRIMARY KEY);
    "#};

    api.raw_cmd(setup).await;

    let expected = expect![[r#"
        generator client {
          provider = "prisma-client-js"
        }

        datasource db {
          provider = "postgresql"
          url      = "env(TEST_DATABASE_URL)"
          schemas  = ["private", "public"]
        }

        model A {
          id Int @id

          @@schema("private")
        }
    "#]];

    api.expect_datamodel(&expected).await;

    let expected = expect![""];
    api.expect_warnings(&expected).await;

    Ok(())
}

#[test_connector(tags(Postgres), exclude(CockroachDb), preview_features("views"))]
async fn ignore_docs_only_added_once(api: &mut TestApi) -> TestResult {
    let setup = indoc! {r#"
        CREATE VIEW "A" AS SELECT 1 AS id;
    "#};

    api.raw_cmd(setup).await;

    let input_dm = indoc! {r#"
        /// The underlying view does not contain a valid unique identifier and can therefore currently not be handled by Prisma Client.
        view A {
          id Int?
        }
    "#};

    let expectation = expect![[r#"
        /// The underlying view does not contain a valid unique identifier and can therefore currently not be handled by Prisma Client.
        view A {
          id Int?
        }
    "#]];

    api.expect_re_introspected_datamodel(input_dm, expectation).await;

    let expectation = expect![""];
    api.expect_re_introspect_warnings(input_dm, expectation).await;

    Ok(())
}

#[test_connector(tags(Postgres), exclude(CockroachDb), preview_features("views"))]
async fn reserved_name_docs_are_only_added_once(api: &mut TestApi) -> TestResult {
    let setup = indoc! {r#"
        CREATE VIEW "if" AS SELECT 1 AS id;
    "#};

    api.raw_cmd(setup).await;

    let input_dm = indoc! {r#"
        /// This view has been renamed to Renamedif during introspection, because the original name if is reserved.
        view Renamedif {
          id Int?

          @@map("if")
          @@ignore
        }
    "#};

    let expectation = expect![[r#"
        /// This view has been renamed to Renamedif during introspection, because the original name if is reserved.
        view Renamedif {
          id Int?

          @@map("if")
          @@ignore
        }
    "#]];

    api.expect_re_introspected_datamodel(input_dm, expectation).await;

    let expectation = expect![[r#"
        *** WARNING ***

        These views were enriched with `@@map` information taken from the previous Prisma schema:
          - "Renamedif"
    "#]];

    api.expect_re_introspect_warnings(input_dm, expectation).await;

    Ok(())
}
