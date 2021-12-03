use indoc::indoc;
use introspection_engine_tests::test_api::*;

#[test_connector(tags(Mysql))]
async fn a_table_with_non_id_autoincrement(api: &TestApi) -> TestResult {
    let setup = r#"
        CREATE TABLE `Test` (
            `id` INTEGER PRIMARY KEY,
            `authorId` INTEGER AUTO_INCREMENT UNIQUE
        );
    "#;

    api.raw_cmd(setup).await;

    let expected = expect![[r#"
        model Test {
          id       Int @id
          authorId Int @unique(map: "authorId") @default(autoincrement())
        }
    "#]];

    expected.assert_eq(&api.introspect_dml().await?);

    Ok(())
}

#[test_connector(tags(Mysql8), preview_features("extendedIndexes"))]
async fn a_table_with_length_prefixed_primary_key(api: &TestApi) -> TestResult {
    let setup = indoc! {r#"
        CREATE TABLE `A` (
            `id` TEXT NOT NULL,
            CONSTRAINT A_id_pkey PRIMARY KEY (id(30))
        )
    "#};

    api.raw_cmd(setup).await;

    let expected = expect![[r#"
        model A {
          id String @id(length: 30) @db.Text
        }
    "#]];

    expected.assert_eq(&api.introspect_dml().await?);

    Ok(())
}

#[test_connector(tags(Mysql8), preview_features("extendedIndexes"))]
async fn a_table_with_length_prefixed_unique(api: &TestApi) -> TestResult {
    let setup = indoc! {r#"
        CREATE TABLE `A` (
            `id` INT  PRIMARY KEY,
            `a`  TEXT NOT NULL,
            CONSTRAINT A_a_key UNIQUE (a(30))
        )
    "#};

    api.raw_cmd(setup).await;

    let expected = expect![[r#"
        model A {
          id Int    @id
          a  String @unique(length: 30) @db.Text
        }
    "#]];

    expected.assert_eq(&api.introspect_dml().await?);

    Ok(())
}

#[test_connector(tags(Mysql8), preview_features("extendedIndexes"))]
async fn a_table_with_length_prefixed_compound_unique(api: &TestApi) -> TestResult {
    let setup = indoc! {r#"
        CREATE TABLE `A` (
            `id` INT  PRIMARY KEY,
            `a`  TEXT NOT NULL,
            `b`  TEXT NOT NULL,
            CONSTRAINT A_a_b_key UNIQUE (a(30), b(20))
        )
    "#};

    api.raw_cmd(setup).await;

    let expected = expect![[r#"
        model A {
          id Int    @id
          a  String @db.Text
          b  String @db.Text

          @@unique([a(length: 30), b(length: 20)])
        }
    "#]];

    expected.assert_eq(&api.introspect_dml().await?);

    Ok(())
}

#[test_connector(tags(Mysql8), preview_features("extendedIndexes"))]
async fn a_table_with_length_prefixed_index(api: &TestApi) -> TestResult {
    let setup = indoc! {r#"
        CREATE TABLE `A` (
            `id` INT  PRIMARY KEY,
            `a`  TEXT NOT NULL,
            `b`  TEXT NOT NULL
        );
        
        CREATE INDEX A_a_b_idx ON `A` (a(30), b(20));
    "#};

    api.raw_cmd(setup).await;

    let expected = expect![[r#"
        model A {
          id Int    @id
          a  String @db.Text
          b  String @db.Text

          @@index([a(length: 30), b(length: 20)])
        }
    "#]];

    expected.assert_eq(&api.introspect_dml().await?);

    Ok(())
}

#[test_connector(tags(Mysql8), preview_features("extendedIndexes"))]
async fn a_table_with_descending_index(api: &TestApi) -> TestResult {
    let setup = indoc! {r#"
        CREATE TABLE `A` (
            `id` INT  PRIMARY KEY,
            `a`  INT NOT NULL,
            `b`  INT NOT NULL
        );
        
        CREATE INDEX A_a_b_idx ON `A` (a ASC, b DESC);
    "#};

    api.raw_cmd(setup).await;

    let expected = expect![[r#"
        model A {
          id Int @id
          a  Int
          b  Int

          @@index([a, b(sort: Desc)])
        }
    "#]];

    expected.assert_eq(&api.introspect_dml().await?);

    Ok(())
}

#[test_connector(tags(Mysql8), preview_features("extendedIndexes"))]
async fn a_table_with_descending_unique(api: &TestApi) -> TestResult {
    let setup = indoc! {r#"
        CREATE TABLE `A` (
            `id` INT  PRIMARY KEY,
            `a`  INT NOT NULL,
            `b`  INT NOT NULL
        );
        
        CREATE UNIQUE INDEX A_a_b_key ON `A` (a ASC, b DESC);
    "#};

    api.raw_cmd(setup).await;

    let expected = expect![[r#"
        model A {
          id Int @id
          a  Int
          b  Int

          @@unique([a, b(sort: Desc)])
        }
    "#]];

    expected.assert_eq(&api.introspect_dml().await?);

    Ok(())
}

#[test_connector(tags(Mysql), preview_features("fullTextIndex"))]
async fn a_table_with_fulltext_index(api: &TestApi) -> TestResult {
    let setup = indoc! {r#"
        CREATE TABLE `A` (
            `id` INT          PRIMARY KEY,
            `a`  VARCHAR(255) NOT NULL,
            `b`  TEXT         NOT NULL
        );
        
        CREATE FULLTEXT INDEX A_a_b_idx ON `A` (a, b);
    "#};

    api.raw_cmd(setup).await;

    let expected = expect![[r#"
        model A {
          id Int    @id
          a  String @db.VarChar(255)
          b  String @db.Text

          @@fulltext([a, b])
        }
    "#]];

    expected.assert_eq(&api.introspect_dml().await?);

    Ok(())
}

#[test_connector(tags(Mysql), preview_features("fullTextIndex"))]
async fn a_table_with_fulltext_index_with_custom_name(api: &TestApi) -> TestResult {
    let setup = indoc! {r#"
        CREATE TABLE `A` (
            `id` INT          PRIMARY KEY,
            `a`  VARCHAR(255) NOT NULL,
            `b`  TEXT         NOT NULL
        );
        
        CREATE FULLTEXT INDEX custom_name ON `A` (a, b);
    "#};

    api.raw_cmd(setup).await;

    let expected = expect![[r#"
        model A {
          id Int    @id
          a  String @db.VarChar(255)
          b  String @db.Text

          @@fulltext([a, b], map: "custom_name")
        }
    "#]];

    expected.assert_eq(&api.introspect_dml().await?);

    Ok(())
}

#[test_connector(tags(Mysql))]
async fn a_table_with_fulltext_index_without_preview_flag(api: &TestApi) -> TestResult {
    let setup = indoc! {r#"
        CREATE TABLE `A` (
            `id` INT          PRIMARY KEY,
            `a`  VARCHAR(255) NOT NULL,
            `b`  TEXT         NOT NULL
        );
        
        CREATE FULLTEXT INDEX A_a_b_idx ON `A` (a, b);
    "#};

    api.raw_cmd(setup).await;

    let expected = expect![[r#"
        model A {
          id Int    @id
          a  String @db.VarChar(255)
          b  String @db.Text

          @@index([a, b])
        }
    "#]];

    expected.assert_eq(&api.introspect_dml().await?);

    Ok(())
}

#[test_connector(tags(Mysql))]
async fn datetime_defaults_dbgenerated(api: &TestApi) -> TestResult {
    let setup = indoc! {r#"
        CREATE TABLE `albums` (
            `id` int(11) NOT NULL AUTO_INCREMENT,
            `updated_at` datetime NOT NULL DEFAULT now(),
            `deleted_at` datetime NOT NULL DEFAULT '1970-01-01 00:00:00',
            PRIMARY KEY (`id`)
        ) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;
    "#};

    api.raw_cmd(setup).await;

    let expected = expect![[r#"
        model albums {
          id         Int      @id @default(autoincrement())
          updated_at DateTime @default(now()) @db.DateTime(0)
          deleted_at DateTime @default(dbgenerated("'1970-01-01 00:00:00'")) @db.DateTime(0)
        }
    "#]];

    expected.assert_eq(&api.introspect_dml().await?);

    Ok(())
}

#[test_connector(tags(Mysql))]
async fn date_defaults_dbgenerated(api: &TestApi) -> TestResult {
    let setup = indoc! {r#"
        CREATE TABLE `albums` (
            `id` int(11) NOT NULL AUTO_INCREMENT,
            `deleted_at` date NOT NULL DEFAULT '1970-01-01',
            PRIMARY KEY (`id`)
        ) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;
    "#};

    api.raw_cmd(setup).await;

    let expected = expect![[r#"
        model albums {
          id         Int      @id @default(autoincrement())
          deleted_at DateTime @default(dbgenerated("'1970-01-01'")) @db.Date
        }
    "#]];

    expected.assert_eq(&api.introspect_dml().await?);

    Ok(())
}

#[test_connector(tags(Mysql))]
async fn time_defaults_dbgenerated(api: &TestApi) -> TestResult {
    let setup = indoc! {r#"
        CREATE TABLE `albums` (
            `id` int(11) NOT NULL AUTO_INCREMENT,
            `deleted_at` time NOT NULL DEFAULT '16:20:00',
            PRIMARY KEY (`id`)
        ) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;
    "#};

    api.raw_cmd(setup).await;

    let expected = expect![[r#"
        model albums {
          id         Int      @id @default(autoincrement())
          deleted_at DateTime @default(dbgenerated("'16:20:00'")) @db.Time(0)
        }
    "#]];

    expected.assert_eq(&api.introspect_dml().await?);

    Ok(())
}
