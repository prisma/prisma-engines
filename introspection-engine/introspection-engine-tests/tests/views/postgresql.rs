use indoc::indoc;
use introspection_engine_tests::test_api::*;

#[test_connector(tags(Postgres), exclude(CockroachDb))]
async fn preview_feature_is_required(api: &TestApi) -> TestResult {
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

#[test_connector(tags(Postgres), exclude(CockroachDb), preview_features("views"))]
async fn simple_view_from_one_table(api: &TestApi) -> TestResult {
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

        /// The underlying view does not contain a valid unique identifier and can therefore currently not be handled by the Prisma Client.
        view Schwuser {
          id         Int?
          first_name String? @db.VarChar(255)
          last_name  String? @db.VarChar(255)

          @@ignore
        }
    "#]];

    api.expect_datamodel(&expected).await;

    Ok(())
}

#[test_connector(tags(Postgres), exclude(CockroachDb), preview_features("views"))]
async fn simple_view_from_two_tables(api: &TestApi) -> TestResult {
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

        /// The underlying view does not contain a valid unique identifier and can therefore currently not be handled by the Prisma Client.
        view Schwuser {
          id           Int?
          name         String?
          introduction String?

          @@ignore
        }
    "#]];

    api.expect_datamodel(&expected).await;

    Ok(())
}

#[test_connector(tags(Postgres), exclude(CockroachDb), preview_features("views"))]
async fn re_intro_keeps_column_arity_and_unique(api: &TestApi) -> TestResult {
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
async fn re_intro_keeps_id(api: &TestApi) -> TestResult {
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
async fn re_intro_keeps_compound_unique(api: &TestApi) -> TestResult {
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
