use indoc::indoc;
use introspection_engine_tests::test_api::*;

#[test_connector(tags(Sqlite), preview_features("views"))]
async fn basic_view_intro(api: &TestApi) -> TestResult {
    let setup = indoc! {r#"
        CREATE TABLE User (
            id INT NOT NULL PRIMARY KEY,
            first_name VARCHAR(255) NOT NULL,
            last_name VARCHAR(255) NULL
        );

        CREATE VIEW Schwuser AS
            SELECT id, first_name, last_name FROM User;
    "#};

    api.raw_cmd(setup).await;

    let expected = expect![[r#"
        generator client {
          provider        = "prisma-client-js"
          previewFeatures = ["views"]
        }

        datasource db {
          provider = "sqlite"
          url      = "env(TEST_DATABASE_URL)"
        }

        model User {
          id         Int     @id
          first_name String
          last_name  String?
        }

        /// The underlying view does not contain a valid unique identifier and can therefore currently not be handled by the Prisma Client.
        view Schwuser {
          id         Int?
          first_name String?
          last_name  String?

          @@ignore
        }
    "#]];

    api.expect_datamodel(&expected).await;

    let expected = expect![[r#"
        SELECT
          id,
          first_name,
          last_name
        FROM
          User;"#]];

    api.expect_view_definition(api.schema_name(), "Schwuser", &expected)
        .await;

    Ok(())
}

#[test_connector(tags(Sqlite), preview_features("views"))]
async fn re_intro_keeps_column_arity_and_unique(api: &TestApi) -> TestResult {
    let setup = indoc! {r#"
        CREATE TABLE User (
            id INT NOT NULL PRIMARY KEY,
            first_name VARCHAR(255) NOT NULL,
            last_name VARCHAR(255) NULL
        );

        CREATE VIEW Schwuser AS
            SELECT id, first_name, last_name FROM User;
    "#};

    api.raw_cmd(setup).await;

    let input = indoc! {r#"
        model User {
          id         Int     @id
          first_name String
          last_name  String?
        }

        view Schwuser {
          id         Int     @unique
          first_name String
          last_name  String?
        }  
    "#};

    let expected = expect![[r#"
        model User {
          id         Int     @id
          first_name String
          last_name  String?
        }

        view Schwuser {
          id         Int     @unique
          first_name String
          last_name  String?
        }
    "#]];

    api.expect_re_introspected_datamodel(input, expected).await;

    Ok(())
}

#[test_connector(tags(Sqlite), preview_features("views"))]
async fn defaults_are_introspected(api: &TestApi) -> TestResult {
    let setup = indoc! {r#"
        CREATE TABLE A (id INT NOT NULL PRIMARY KEY, val INT DEFAULT 2);
        CREATE VIEW B AS SELECT id, val FROM A;
    "#};

    api.raw_cmd(setup).await;

    let expected = expect![[r#"
        generator client {
          provider        = "prisma-client-js"
          previewFeatures = ["views"]
        }

        datasource db {
          provider = "sqlite"
          url      = "env(TEST_DATABASE_URL)"
        }

        model A {
          id  Int  @id
          val Int? @default(2)
        }

        /// The underlying view does not contain a valid unique identifier and can therefore currently not be handled by the Prisma Client.
        view B {
          id  Int?
          val Int?

          @@ignore
        }
    "#]];

    api.expect_datamodel(&expected).await;

    Ok(())
}
