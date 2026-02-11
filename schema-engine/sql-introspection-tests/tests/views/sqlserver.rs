use indoc::indoc;
use sql_introspection_tests::test_api::*;

#[test_connector(tags(Mssql), preview_features("views"))]
async fn simple_view_from_one_table(api: &mut TestApi) -> TestResult {
    let setup = indoc! {r#"
        CREATE TABLE A (
            id INT NOT NULL,
            first_name VARCHAR(255) NOT NULL,
            last_name VARCHAR(255) NULL,
            CONSTRAINT A_pkey PRIMARY KEY (id)
        );
    "#};

    api.raw_cmd(setup).await;

    let setup = indoc! {r#"
        CREATE VIEW B AS SELECT id, first_name, last_name FROM A;
    "#};

    api.raw_cmd(setup).await;

    let expected = expect![[r#"
        generator client {
          provider        = "prisma-client"
          previewFeatures = ["views"]
        }

        datasource db {
          provider = "sqlserver"
        }

        model A {
          id         Int     @id
          first_name String  @db.VarChar(255)
          last_name  String? @db.VarChar(255)
        }

        view B {
          id         Int
          first_name String  @db.VarChar(255)
          last_name  String? @db.VarChar(255)
        }
    "#]];

    api.expect_datamodel(&expected).await;

    let expected = expect![[r#"
        SELECT
          id,
          first_name,
          last_name
        FROM
          A;"#]];

    api.expect_view_definition("B", &expected).await;

    api.expect_no_warnings().await;

    Ok(())
}

#[test_connector(tags(Mssql), preview_features("views"))]
async fn simple_view_with_cte(api: &mut TestApi) -> TestResult {
    let setup = indoc! {r#"
        CREATE VIEW A AS WITH foo AS (SELECT 1 AS bar) SELECT bar FROM foo;
    "#};

    api.raw_cmd(setup).await;

    let expected = expect![[r#"
        generator client {
          provider        = "prisma-client"
          previewFeatures = ["views"]
        }

        datasource db {
          provider = "sqlserver"
        }

        view A {
          bar Int
        }
    "#]];

    api.expect_datamodel(&expected).await;

    let expected = expect![[r#"
        WITH foo AS (
          SELECT
            1 AS bar
        )
        SELECT
          bar
        FROM
          foo;"#]];

    api.expect_view_definition("A", &expected).await;

    Ok(())
}

#[test_connector(tags(Mssql), preview_features("views"))]
async fn simple_view_from_two_tables(api: &mut TestApi) -> TestResult {
    let setup = indoc! {r#"
        CREATE TABLE A (
            id INT,
            first_name VARCHAR(255) NOT NULL,
            last_name VARCHAR(255) NULL,
            CONSTRAINT A_pkey PRIMARY KEY (id)
        );

        CREATE TABLE B (
            user_id INT,
            introduction VARCHAR(MAX),
            CONSTRAINT Profile_User_fkey FOREIGN KEY (user_id) REFERENCES A(id) ON DELETE CASCADE ON UPDATE CASCADE,
            CONSTRAINT B_pkey PRIMARY KEY (user_id)
        );
    "#};

    api.raw_cmd(setup).await;

    let setup = indoc! {r#"
        CREATE VIEW AB AS
            SELECT
                a.id,
                CONCAT(a.first_name, ' ', a.last_name) AS name,
                b.introduction
            FROM A a
            INNER JOIN B b ON a.id = b.user_id;
    "#};

    api.raw_cmd(setup).await;

    let expected = expect![[r#"
        generator client {
          provider        = "prisma-client"
          previewFeatures = ["views"]
        }

        datasource db {
          provider = "sqlserver"
        }

        model A {
          id         Int     @id
          first_name String  @db.VarChar(255)
          last_name  String? @db.VarChar(255)
          B          B?
        }

        model B {
          user_id      Int     @id
          introduction String? @db.VarChar(Max)
          A            A       @relation(fields: [user_id], references: [id], onDelete: Cascade, map: "Profile_User_fkey")
        }

        view AB {
          id           Int
          name         String  @db.VarChar(511)
          introduction String? @db.VarChar(Max)
        }
    "#]];

    api.expect_datamodel(&expected).await;

    let expected = expect![[r#"
        SELECT
          a.id,
          CONCAT(a.first_name, ' ', a.last_name) AS name,
          b.introduction
        FROM
          A a
          INNER JOIN B b ON a.id = b.user_id;"#]];

    api.expect_view_definition("AB", &expected).await;

    Ok(())
}

#[test_connector(tags(Mssql), preview_features("views"))]
async fn re_intro_keeps_view_to_view_relations(api: &mut TestApi) -> TestResult {
    let setup = indoc! {r#"
        CREATE VIEW A AS SELECT 1 AS id;
    "#};

    api.raw_cmd(setup).await;

    let setup = indoc! {r#"
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
          id Int @unique
          b  B[]
        }

        view B {
          id   Int @unique
          a_id Int
          a    A?  @relation(fields: [a_id], references: [id])
        }
    "#]];

    api.expect_re_introspected_datamodel(input, expected).await;

    Ok(())
}

#[test_connector(tags(Mssql), preview_features("views"))]
async fn views_cannot_have_default_values(api: &mut TestApi) -> TestResult {
    let setup = indoc! {r#"
        CREATE TABLE A (id INT CONSTRAINT A_pkey PRIMARY KEY, val INT CONSTRAINT A_val_df DEFAULT 2);
    "#};

    api.raw_cmd(setup).await;

    let setup = indoc! {r#"
        CREATE VIEW B AS SELECT id, val FROM A;
    "#};

    api.raw_cmd(setup).await;

    let expected = expect![[r#"
        generator client {
          provider        = "prisma-client"
          previewFeatures = ["views"]
        }

        datasource db {
          provider = "sqlserver"
        }

        model A {
          id  Int  @id
          val Int? @default(2)
        }

        view B {
          id  Int
          val Int?
        }
    "#]];

    api.expect_datamodel(&expected).await;

    Ok(())
}

#[test_connector(tags(Mssql), preview_features("views"))]
async fn prisma_defaults_are_kept(api: &mut TestApi) -> TestResult {
    let setup = indoc! {r#"
        CREATE TABLE A (id INT CONSTRAINT A_pkey PRIMARY KEY, val VARCHAR(255));
    "#};

    api.raw_cmd(setup).await;

    let setup = indoc! {r#"
        CREATE VIEW B AS SELECT id, val FROM A;
    "#};

    api.raw_cmd(setup).await;

    let input = indoc! {r#"
        model A {
          id  Int     @id
          val String? @db.VarChar(255)
        }

        view B {
          id  Int
          val String? @db.VarChar(255) @default(cuid())
        }
    "#};

    let expected = expect![[r#"
        model A {
          id  Int     @id
          val String? @db.VarChar(255)
        }

        view B {
          id  Int
          val String? @default(cuid()) @db.VarChar(255)
        }
    "#]];

    api.expect_re_introspected_datamodel(input, expected).await;

    Ok(())
}
