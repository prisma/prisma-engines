use datamodel::dml::Datamodel;
use indoc::{formatdoc, indoc};
use introspection_connector::{IntrospectionConnector, IntrospectionContext};
use introspection_engine_tests::test_api::*;
use sql_introspection_connector::SqlIntrospectionConnector;
use url::Url;

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

#[test_connector(tags(Mysql8))]
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

#[test_connector(tags(Mysql8))]
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

#[test_connector(tags(Mysql8))]
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

#[test_connector(tags(Mysql8))]
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

#[test_connector(tags(Mysql8))]
async fn a_table_with_non_length_prefixed_index(api: &TestApi) -> TestResult {
    let setup = indoc! {r#"
        CREATE TABLE `A` (
            `id` INT  PRIMARY KEY,
            `a`  VARCHAR(190) NOT NULL,
            `b`  VARCHAR(192) NOT NULL
        );
        
        CREATE INDEX A_a_idx ON `A` (a);
        CREATE INDEX A_b_idx ON `A` (b(191));
    "#};

    api.raw_cmd(setup).await;

    let expected = expect![[r#"
        model A {
          id Int    @id
          a  String @db.VarChar(190)
          b  String @db.VarChar(192)

          @@index([a])
          @@index([b(length: 191)])
        }
    "#]];

    expected.assert_eq(&api.introspect_dml().await?);

    Ok(())
}

#[test_connector(tags(Mysql8))]
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

#[test_connector(tags(Mysql8))]
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

#[test_connector(tags(Mysql), exclude(Mariadb))]
async fn date_time_defaults(api: &TestApi) -> TestResult {
    let setup = indoc! {r#"
        CREATE TABLE `A` (
            id INT PRIMARY KEY auto_increment,
            d1 DATE DEFAULT '2020-01-01',
            d2 DATETIME DEFAULT '2038-01-19 03:14:08',
            d3 TIME DEFAULT '16:20:00'
        )
    "#};

    api.raw_cmd(setup).await;

    let expected = expect![[r#"
        model A {
          id Int       @id @default(autoincrement())
          d1 DateTime? @default(dbgenerated("'2020-01-01'")) @db.Date
          d2 DateTime? @default(dbgenerated("'2038-01-19 03:14:08'")) @db.DateTime(0)
          d3 DateTime? @default(dbgenerated("'16:20:00'")) @db.Time(0)
        }
    "#]];

    expected.assert_eq(&api.introspect_dml().await?);

    Ok(())
}

#[test_connector(tags(Mariadb))]
async fn date_time_defaults_mariadb(api: &TestApi) -> TestResult {
    let setup = indoc! {r#"
        CREATE TABLE `A` (
            id INT PRIMARY KEY auto_increment,
            d1 DATE DEFAULT '2020-01-01',
            d2 DATETIME DEFAULT '2038-01-19 03:14:08',
            d3 TIME DEFAULT '16:20:00'
        )
    "#};

    api.raw_cmd(setup).await;

    let expected = expect![[r#"
        model A {
          id Int       @id @default(autoincrement())
          d1 DateTime? @default(dbgenerated("('2020-01-01')")) @db.Date
          d2 DateTime? @default(dbgenerated("('2038-01-19 03:14:08')")) @db.DateTime(0)
          d3 DateTime? @default(dbgenerated("('16:20:00')")) @db.Time(0)
        }
    "#]];

    expected.assert_eq(&api.introspect_dml().await?);

    Ok(())
}

#[test_connector(tags(Mysql8), exclude(Vitess))]
async fn missing_select_rights(api: &TestApi) -> TestResult {
    let setup = formatdoc!(
        r#"
        CREATE TABLE `A` (
            id INT PRIMARY KEY auto_increment,
            val INT NOT NULL,
            data VARCHAR(20) NULL
        );

        CREATE INDEX `test_index` ON `A` (`data`);
        CREATE UNIQUE INDEX `test_unique` ON `A` (`val`);

        DROP USER IF EXISTS 'jeffrey'@'%';
        CREATE USER 'jeffrey'@'%' IDENTIFIED BY 'password';
        GRANT USAGE, CREATE ON TABLE `{}`.* TO 'jeffrey'@'%';
        FLUSH PRIVILEGES;
    "#,
        api.schema_name()
    );

    api.raw_cmd(&setup).await;

    let expected = expect![[r#"
        model A {
          id   Int     @id @default(autoincrement())
          val  Int     @unique(map: "test_unique")
          data String? @db.VarChar(20)

          @@index([data], map: "test_index")
        }
    "#]];

    expected.assert_eq(&api.introspect_dml().await?);

    let mut url: Url = api.connection_string().parse()?;
    url.set_username("jeffrey").unwrap();
    url.set_password(Some("password")).unwrap();

    let conn = SqlIntrospectionConnector::new(url.as_ref(), Default::default()).await?;

    let datasource = formatdoc!(
        r#"
        datasource db {{
          provider = "mysql"
          url      = "{url}"
        }}
    "#
    );

    let config = datamodel::parse_configuration(&datasource).unwrap();

    let ctx = IntrospectionContext {
        source: config.subject.datasources.into_iter().next().unwrap(),
        composite_type_depth: Default::default(),
        preview_features: Default::default(),
    };

    let res = conn.introspect(&Datamodel::new(), ctx).await.unwrap();
    assert!(res.data_model.is_empty());

    Ok(())
}
