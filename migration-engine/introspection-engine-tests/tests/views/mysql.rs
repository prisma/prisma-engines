use indoc::indoc;
use introspection_engine_tests::test_api::*;

#[test_connector(tags(Mysql), exclude(Vitess), preview_features("views"))]
async fn simple_view_from_one_table(api: &TestApi) -> TestResult {
    let setup = indoc! {r#"
        CREATE TABLE A (
            id INT PRIMARY KEY,
            first_name VARCHAR(255) NOT NULL,
            last_name VARCHAR(255) NULL
        );

        CREATE VIEW B AS SELECT id, first_name, last_name FROM A;
    "#};

    api.raw_cmd(setup).await;

    let expected = expect![[r#"
        generator client {
          provider        = "prisma-client-js"
          previewFeatures = ["views"]
        }

        datasource db {
          provider = "mysql"
          url      = "env(TEST_DATABASE_URL)"
        }

        model A {
          id         Int     @id
          first_name String  @db.VarChar(255)
          last_name  String? @db.VarChar(255)
        }

        /// The underlying view does not contain a valid unique identifier and can therefore currently not be handled by the Prisma Client.
        view B {
          id         Int
          first_name String  @db.VarChar(255)
          last_name  String? @db.VarChar(255)

          @@ignore
        }
    "#]];

    api.expect_datamodel(&expected).await;

    let expected = expect![[r#"
        SELECT
          `simple_view_from_one_table`.`A`.`id` AS `id`,
          `simple_view_from_one_table`.`A`.`first_name` AS `first_name`,
          `simple_view_from_one_table`.`A`.`last_name` AS `last_name`
        FROM
          `simple_view_from_one_table`.`A`"#]];

    api.expect_view_definition(api.schema_name(), "B", &expected).await;

    let expected = expect![[r#"
        [
          {
            "code": 24,
            "message": "The following views were ignored as they do not have a valid unique identifier or id. This is currently not supported by the Prisma Client. Please refer to the documentation on defining unique identifiers in views: https://pris.ly/d/view-identifiers",
            "affected": [
              {
                "view": "B"
              }
            ]
          }
        ]"#]];

    api.expect_warnings(&expected).await;

    Ok(())
}

#[test_connector(tags(Mysql), exclude(Vitess, Mariadb), preview_features("views"))]
async fn simple_view_from_two_tables(api: &TestApi) -> TestResult {
    let setup = indoc! {r#"
        CREATE TABLE User (
            id INT PRIMARY KEY,
            first_name VARCHAR(255) NOT NULL,
            last_name VARCHAR(255) NULL
        );

        CREATE TABLE Profile (
            user_id INT PRIMARY KEY,
            introduction TEXT,
            CONSTRAINT Profile_User_fkey FOREIGN KEY (user_id) REFERENCES User(id) ON DELETE CASCADE ON UPDATE CASCADE
        );

        CREATE VIEW Schwuser AS
            SELECT
                u.id,
                CONCAT(u.first_name, ' ', u.last_name) AS name,
                p.introduction
            FROM User u
            INNER JOIN Profile p ON u.id = p.user_id;
    "#};

    api.raw_cmd(setup).await;

    let expected = expect![[r#"
        generator client {
          provider        = "prisma-client-js"
          previewFeatures = ["views"]
        }

        datasource db {
          provider = "mysql"
          url      = "env(TEST_DATABASE_URL)"
        }

        model Profile {
          user_id      Int     @id
          introduction String? @db.Text
          User         User    @relation(fields: [user_id], references: [id], onDelete: Cascade, map: "Profile_User_fkey")
        }

        model User {
          id         Int      @id
          first_name String   @db.VarChar(255)
          last_name  String?  @db.VarChar(255)
          Profile    Profile?
        }

        /// The underlying view does not contain a valid unique identifier and can therefore currently not be handled by the Prisma Client.
        view Schwuser {
          id           Int
          name         String? @db.VarChar(511)
          introduction String? @db.Text

          @@ignore
        }
    "#]];

    api.expect_datamodel(&expected).await;

    let expected = expect![[r#"
        SELECT
          `u`.`id` AS `id`,
          concat(`u`.`first_name`, ' ', `u`.`last_name`) AS `name`,
          `p`.`introduction` AS `introduction`
        FROM
          (
            `simple_view_from_two_tables`.`User` `u`
            JOIN `simple_view_from_two_tables`.`Profile` `p` ON((`u`.`id` = `p`.`user_id`))
          )"#]];

    api.expect_view_definition(api.schema_name(), "Schwuser", &expected)
        .await;

    Ok(())
}

#[test_connector(tags(Mysql), exclude(Vitess), preview_features("views"))]
async fn re_intro_keeps_view_uniques(api: &TestApi) -> TestResult {
    let setup = indoc! {r#"
        CREATE TABLE User (
            id INT PRIMARY KEY,
            first_name VARCHAR(255) NOT NULL,
            last_name VARCHAR(255) NOT NULL
        );

        CREATE VIEW Schwuser AS
            SELECT id, first_name, last_name FROM User;
    "#};

    api.raw_cmd(setup).await;

    let input = indoc! {r#"
        model User {
          id         Int     @id @default(autoincrement())
          first_name String? @db.VarChar(255)
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
          id         Int    @id
          first_name String @db.VarChar(255)
          last_name  String @db.VarChar(255)
        }

        view Schwuser {
          id         Int    @unique
          first_name String @db.VarChar(255)
          last_name  String @db.VarChar(255)
        }
    "#]];

    api.expect_re_introspected_datamodel(input, expected).await;

    Ok(())
}

#[test_connector(tags(Mysql), exclude(Vitess), preview_features("views"))]
async fn re_intro_keeps_id(api: &TestApi) -> TestResult {
    let setup = indoc! {r#"
        CREATE TABLE User (
            id INT PRIMARY KEY,
            first_name VARCHAR(255) NOT NULL,
            last_name VARCHAR(255) NULL
        );

        CREATE VIEW Schwuser AS
            SELECT id, first_name, last_name FROM User;
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
          id         Int     @id
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

#[test_connector(tags(Mysql), exclude(Vitess), preview_features("views"))]
async fn re_intro_keeps_compound_unique(api: &TestApi) -> TestResult {
    let setup = indoc! {r#"
        CREATE TABLE User (
            side_a INT NOT NULL,
            side_b INT NOT NULL,
            first_name VARCHAR(255) NOT NULL,
            last_name VARCHAR(255) NULL,
            CONSTRAINT User_pkey PRIMARY KEY (side_a, side_b)
        );

        CREATE VIEW Schwuser AS
            SELECT side_a, side_b, first_name, last_name FROM User;
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

#[test_connector(tags(Mysql), exclude(Vitess), preview_features("views"))]
async fn re_intro_keeps_view_to_view_relations(api: &TestApi) -> TestResult {
    let setup = indoc! {r#"
        CREATE VIEW A AS SELECT 1 AS id;
        CREATE VIEW B AS SELECT 2 AS id, 1 AS a_id;
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
          id Int @unique @default(0)
          b  B[]
        }

        view B {
          id   Int @unique @default(0)
          a_id Int @default(0)
          a    A?  @relation(fields: [a_id], references: [id])
        }
    "#]];

    api.expect_re_introspected_datamodel(input, expected).await;

    Ok(())
}

#[test_connector(tags(Mysql), exclude(Vitess), preview_features("views"))]
async fn defaults_are_introspected(api: &TestApi) -> TestResult {
    let setup = indoc! {r#"
        CREATE TABLE A (id INT PRIMARY KEY, val INT DEFAULT 2);
        CREATE VIEW B AS SELECT id, val FROM A;
    "#};

    api.raw_cmd(setup).await;

    let expected = expect![[r#"
        generator client {
          provider        = "prisma-client-js"
          previewFeatures = ["views"]
        }

        datasource db {
          provider = "mysql"
          url      = "env(TEST_DATABASE_URL)"
        }

        model A {
          id  Int  @id
          val Int? @default(2)
        }

        /// The underlying view does not contain a valid unique identifier and can therefore currently not be handled by the Prisma Client.
        view B {
          id  Int
          val Int? @default(2)

          @@ignore
        }
    "#]];

    api.expect_datamodel(&expected).await;

    Ok(())
}

#[test_connector(tags(Mysql8), exclude(Vitess), preview_features("views"))]
async fn views_are_rendered_with_enums(api: &TestApi) -> TestResult {
    let setup = indoc! {r#"
        CREATE TABLE A (
            id INT PRIMARY KEY,
            val ENUM('a', 'b')
        );

        CREATE VIEW B AS SELECT id, val from A;
    "#};

    api.raw_cmd(setup).await;

    let expected = expect![[r#"
        generator client {
          provider        = "prisma-client-js"
          previewFeatures = ["views"]
        }

        datasource db {
          provider = "mysql"
          url      = "env(TEST_DATABASE_URL)"
        }

        model A {
          id  Int    @id
          val A_val?
        }

        /// The underlying view does not contain a valid unique identifier and can therefore currently not be handled by the Prisma Client.
        view B {
          id  Int
          val B_val?

          @@ignore
        }

        enum A_val {
          a
          b
        }

        enum B_val {
          a
          b
        }
    "#]];

    api.expect_datamodel(&expected).await;

    Ok(())
}
