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
#[test_connector(tags(Postgres), exclude(Postgres16), exclude(CockroachDb), preview_features("views"))]
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

        /// The underlying view does not contain a valid unique identifier and can therefore currently not be handled by Prisma Client.
        view Schwuser {
          id         Int?
          first_name String? @db.VarChar(255)
          last_name  String? @db.VarChar(255)

          @@ignore
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

    let expected = expect![[r#"
        *** WARNING ***

        The following views were ignored as they do not have a valid unique identifier or id. This is currently not supported by Prisma Client. Please refer to the documentation on defining unique identifiers in views: https://pris.ly/d/view-identifiers
          - "Schwuser"
    "#]];
    api.expect_warnings(&expected).await;

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

        /// The underlying view does not contain a valid unique identifier and can therefore currently not be handled by Prisma Client.
        view Schwuser {
          id         Int?
          first_name String? @db.VarChar(255)
          last_name  String? @db.VarChar(255)

          @@ignore
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

    let expected = expect![[r#"
        *** WARNING ***

        The following views were ignored as they do not have a valid unique identifier or id. This is currently not supported by Prisma Client. Please refer to the documentation on defining unique identifiers in views: https://pris.ly/d/view-identifiers
          - "Schwuser"
    "#]];
    api.expect_warnings(&expected).await;

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

        /// The underlying view does not contain a valid unique identifier and can therefore currently not be handled by Prisma Client.
        view Schwuser {
          id           Int?
          name         String?
          introduction String?

          @@ignore
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
          id         Int     @unique
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
          id         Int     @unique
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
          id  Int @unique
          val Int
        }
    "#};

    let expected = expect![[r#"
        model User {
          id  Int   @id @default(autoincrement())
          val Int[]
        }

        view Schwuser {
          id  Int   @unique
          val Int[]
        }
    "#]];

    api.expect_re_introspected_datamodel(input, expected).await;

    Ok(())
}

#[test_connector(tags(Postgres), exclude(CockroachDb), preview_features("views"))]
async fn re_intro_keeps_id(api: &mut TestApi) -> TestResult {
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
          id         Int     @id
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
          id         Int     @id
          first_name String  @db.VarChar(255)
          last_name  String? @db.VarChar(255)
        }
    "#]];

    api.expect_re_introspected_datamodel(input, expected).await;

    Ok(())
}

#[test_connector(tags(Postgres), exclude(CockroachDb), preview_features("views"))]
async fn re_intro_keeps_compound_unique(api: &mut TestApi) -> TestResult {
    let setup = indoc! {r#"
        CREATE TABLE "User" (
            side_a INT NOT NULL,
            side_b INT NOT NULL,
            first_name VARCHAR(255) NOT NULL,
            last_name VARCHAR(255) NULL,
            CONSTRAINT "User_pkey" PRIMARY KEY (side_a, side_b)
        );

        CREATE VIEW "Schwuser" AS
            SELECT side_a, side_b, first_name, last_name FROM "User";
    "#};

    api.raw_cmd(setup).await;

    let input = indoc! {r#"
        model User {
          side_a     Int
          side_b     Int
          first_name String  @db.VarChar(255)
          last_name  String? @db.VarChar(255)

          @@id([side_a, side_b])
        }

        view Schwuser {
          side_a     Int
          side_b     Int
          first_name String  @db.VarChar(255)
          last_name  String? @db.VarChar(255)

          @@unique([side_a, side_b])
        }  
    "#};

    let expected = expect![[r#"
        model User {
          side_a     Int
          side_b     Int
          first_name String  @db.VarChar(255)
          last_name  String? @db.VarChar(255)

          @@id([side_a, side_b])
        }

        view Schwuser {
          side_a     Int
          side_b     Int
          first_name String  @db.VarChar(255)
          last_name  String? @db.VarChar(255)

          @@unique([side_a, side_b])
        }
    "#]];

    api.expect_re_introspected_datamodel(input, expected).await;

    Ok(())
}

#[test_connector(tags(Postgres), exclude(CockroachDb), preview_features("views"))]
async fn re_intro_keeps_back_relations(api: &mut TestApi) -> TestResult {
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

        CREATE TABLE "Random" (
            id INT PRIMARY KEY,
            view_id INT NOT NULL
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

    let input = indoc! {r#"
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

        model Random {
          id       Int       @id
          view_id  Int?
          schwuser Schwuser? @relation(fields: [view_id], references: [id])
        }

        view Schwuser {
          id         Int      @unique
          first_name String   @db.VarChar(255)
          last_name  String?  @db.VarChar(255)
          random     Random[]
        }  
    "#};

    let expected = expect![[r#"
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

        model Random {
          id       Int       @id
          view_id  Int
          schwuser Schwuser? @relation(fields: [view_id], references: [id])
        }

        view Schwuser {
          id           Int      @unique
          name         String?
          introduction String?
          random       Random[]
        }
    "#]];

    api.expect_re_introspected_datamodel(input, expected).await;

    Ok(())
}

#[test_connector(tags(Postgres), exclude(CockroachDb), preview_features("views"))]
async fn re_intro_keeps_forward_relations(api: &mut TestApi) -> TestResult {
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

        CREATE TABLE "Random" (
            id INT PRIMARY KEY
        );

        CREATE VIEW "Schwuser" AS
            SELECT
                u.id,
                CONCAT(u.first_name, ' ', u.last_name) AS name,
                p.introduction,
                1 AS random_id
            FROM "User" u
            INNER JOIN "Profile" p ON u.id = p.user_id;
    "#};

    api.raw_cmd(setup).await;

    let input = indoc! {r#"
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

        model Random {
          id       Int        @id
          schwuser Schwuser[]
        }

        view Schwuser {
          id         Int     @unique
          first_name String  @db.VarChar(255)
          last_name  String? @db.VarChar(255)
          random_id  Int?
          random     Random? @relation(fields: [random_id], references: [id])
        }  
    "#};

    let expected = expect![[r#"
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

        model Random {
          id       Int        @id
          schwuser Schwuser[]
        }

        view Schwuser {
          id           Int     @unique
          name         String?
          introduction String?
          random_id    Int?
          random       Random? @relation(fields: [random_id], references: [id])
        }
    "#]];

    api.expect_re_introspected_datamodel(input, expected).await;

    Ok(())
}

#[test_connector(tags(Postgres), exclude(CockroachDb), preview_features("views"))]
async fn re_intro_keeps_view_to_view_relations(api: &mut TestApi) -> TestResult {
    let setup = indoc! {r#"
        CREATE VIEW "A" AS SELECT 1 AS id;
        CREATE VIEW "B" AS SELECT 2 AS id, 1 AS a_id;
    "#};

    api.raw_cmd(setup).await;

    let input = indoc! {r#"
        view A {
          id Int @unique
          b  B[]
        }

        view B {
          id   Int  @unique
          a_id Int?
          a    A?   @relation(fields: [a_id], references: [id])
        }
    "#};

    let expected = expect![[r#"
        view A {
          id Int @unique
          b  B[]
        }

        view B {
          id   Int  @unique
          a_id Int?
          a    A?   @relation(fields: [a_id], references: [id])
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
          id Int @unique
        }
    "#};

    let expected = expect![[r#"
        /// I'm a view doc
        view A {
          /// I'm a field doc
          id Int @unique
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
          id Int @unique

          @@ignore
        }
    "#};

    let expected = expect![[r#"
        view A {
          id Int @unique

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
        /// The underlying view does not contain a valid unique identifier and can therefore currently not be handled by Prisma Client.
        view Renamedif {
          id Int?

          @@map("if")
          @@ignore
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
          id Int @unique
        }
    "#};

    let expected = expect![[r#"
        view A {
          id     Int                      @unique
          vector Unsupported("tsvector")?
        }
    "#]];

    api.expect_re_introspected_datamodel(input, expected).await;

    let expected = expect![[r#"
        *** WARNING ***

        The following views were ignored as they do not have a valid unique identifier or id. This is currently not supported by Prisma Client. Please refer to the documentation on defining unique identifiers in views: https://pris.ly/d/view-identifiers
          - "A"

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
          id Int @unique

          @@map("A")
        }
    "#};

    let expected = expect![[r#"
        view B {
          id Int @unique

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
          meow Int @unique @map("id")
        }
    "#};

    let expected = expect![[r#"
        view A {
          meow Int @unique @map("id")
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
    preview_features("views", "multiSchema"),
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
          previewFeatures = ["multiSchema", "views"]
        }

        datasource db {
          provider = "postgresql"
          url      = "env(TEST_DATABASE_URL)"
          schemas  = ["public"]
        }

        /// The underlying view does not contain a valid unique identifier and can therefore currently not be handled by Prisma Client.
        view A {
          id Int?

          @@ignore
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

        /// The underlying view does not contain a valid unique identifier and can therefore currently not be handled by Prisma Client.
        view A {
          id Int? @default(3)

          @@ignore
        }
    "#]];

    api.expect_datamodel(&expected).await;

    Ok(())
}

#[test_connector(tags(Postgres), exclude(CockroachDb), preview_features("views"))]
async fn id_names_are_reintrospected(api: &mut TestApi) -> TestResult {
    let setup = indoc! {r#"
        CREATE VIEW "A" AS SELECT 1 AS a, 2 AS b;
    "#};

    api.raw_cmd(setup).await;

    let input = indoc! {r#"
        view A {
          a Int
          b Int

          @@id([a, b], name: "kekw")
        }   
    "#};

    let expected = expect![[r#"
        view A {
          a Int
          b Int

          @@id([a, b], name: "kekw")
        }
    "#]];

    api.expect_re_introspected_datamodel(input, expected).await;

    let expected = expect![[r#"
        *** WARNING ***

        These views were enriched with custom compound id names taken from the previous Prisma schema:
          - "A"
    "#]];

    api.expect_re_introspect_warnings(input, expected).await;

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

        /// The underlying view does not contain a valid unique identifier and can therefore currently not be handled by Prisma Client.
        view A {
          foo Int?

          /// This field was commented out because of an invalid name. Please provide a valid one that matches [a-zA-Z][a-zA-Z0-9_]*
          // 1 Int? @map("1")
          @@ignore
        }
    "#]];

    api.expect_datamodel(&expected).await;

    let expected = expect![[r#"
        *** WARNING ***

        These fields were commented out because their names are currently not supported by Prisma. Please provide valid ones that match [a-zA-Z][a-zA-Z0-9_]* using the `@map` attribute:
          - View: "A", field(s): ["1"]

        The following views were ignored as they do not have a valid unique identifier or id. This is currently not supported by Prisma Client. Please refer to the documentation on defining unique identifiers in views: https://pris.ly/d/view-identifiers
          - "A"
    "#]];

    api.expect_warnings(&expected).await;

    Ok(())
}

#[test_connector(
    tags(Postgres),
    exclude(CockroachDb),
    preview_features("views", "multiSchema"),
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
          previewFeatures = ["multiSchema", "views"]
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

        /// The underlying view does not contain a valid unique identifier and can therefore currently not be handled by Prisma Client.
        view public_A {
          id Int?

          @@map("A")
          @@ignore
          @@schema("public")
        }
    "#]];

    api.expect_datamodel(&expected).await;

    let expected = expect![[r#"
        *** WARNING ***

        The following views were ignored as they do not have a valid unique identifier or id. This is currently not supported by Prisma Client. Please refer to the documentation on defining unique identifiers in views: https://pris.ly/d/view-identifiers
          - "public_A"

        These items were renamed due to their names being duplicates in the Prisma schema:
          - Type: "model", name: "private_A"
          - Type: "view", name: "public_A"
    "#]];

    api.expect_warnings(&expected).await;

    Ok(())
}

#[test_connector(
    tags(Postgres),
    exclude(CockroachDb),
    preview_features("multiSchema"),
    namespaces("public", "private")
)]
async fn dupe_views_are_not_considered_without_preview_feature(api: &mut TestApi) -> TestResult {
    let setup = indoc! {r#"
        CREATE SCHEMA IF NOT EXISTS "private";
        CREATE VIEW public."A" AS SELECT 1 AS id;
        CREATE TABLE private."A" (id INT PRIMARY KEY);
    "#};

    api.raw_cmd(setup).await;

    let expected = expect![[r#"
        generator client {
          provider        = "prisma-client-js"
          previewFeatures = ["multiSchema"]
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

          @@ignore
        }
    "#};

    let expectation = expect![[r#"
        /// The underlying view does not contain a valid unique identifier and can therefore currently not be handled by Prisma Client.
        view A {
          id Int?

          @@ignore
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
