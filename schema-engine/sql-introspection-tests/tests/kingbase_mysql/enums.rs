use indoc::indoc;
use sql_introspection_tests::test_api::*;
use test_macros::test_connector;

#[test_connector(tags(KingbaseMysql))]
async fn enums_and_scalar_types_are_introspected(api: &mut TestApi) -> TestResult {
    api.raw_cmd(indoc! {r#"
        CREATE TABLE type_samples (
            id INTEGER NOT NULL AUTO_INCREMENT PRIMARY KEY,
            amount DECIMAL(10, 2),
            status ENUM('draft', 'published'),
            labels SET('red', 'blue'),
            payload JSON
        );
    "#})
        .await;

    let datamodel = api.introspect_dml().await?;
    let expected = expect![[r#"
        model type_samples {
          id      Int                  @id @default(autoincrement())
          amount  Decimal?             @db.Decimal(10, 2)
          status  type_samples_status?
          labels  String?
          payload Json?
        }

        enum type_samples_status {
          draft
          published
        }
    "#]];
    expected.assert_eq(&datamodel);

    for _ in 0..4 {
        let result = api.introspect_dml().await?;
        expected.assert_eq(&result);
    }

    Ok(())
}

#[test_connector(tags(KingbaseMysql))]
async fn enum_value_names_are_rendered_safely(api: &mut TestApi) -> TestResult {
    api.raw_cmd(r#"CREATE TABLE enum_names (value ENUM ('123', 'wow', '$§!'));"#)
        .await;

    let expected = expect![[r#"
        /// The underlying table does not contain a valid unique identifier and can therefore currently not be handled by Prisma Client.
        model enum_names {
          value enum_names_value?

          @@ignore
        }

        enum enum_names_value {
          // 123 @map("123")
          wow
          // $§! @map("$§!")
        }
    "#]];

    expected.assert_eq(&api.introspect_dml().await?);
    Ok(())
}

#[test_connector(tags(KingbaseMysql))]
async fn enum_empty_string_defaults_are_preserved(api: &mut TestApi) -> TestResult {
    api.raw_cmd(indoc! {r#"
        CREATE TABLE enum_empty_default (
            id INT NOT NULL AUTO_INCREMENT,
            color ENUM ('black', '') NOT NULL DEFAULT '',
            PRIMARY KEY (id)
        );
    "#})
        .await;

    let expected = expect![[r#"
        model enum_empty_default {
          id    Int                      @id @default(autoincrement())
          color enum_empty_default_color @default(EMPTY_ENUM_VALUE)
        }

        enum enum_empty_default_color {
          black
          EMPTY_ENUM_VALUE @map("")
        }
    "#]];

    expected.assert_eq(&api.introspect_dml().await?);
    Ok(())
}

#[test_connector(tags(KingbaseMysql))]
async fn enum_boolean_like_defaults_are_not_coerced(api: &mut TestApi) -> TestResult {
    api.raw_cmd(indoc! {r#"
        CREATE TABLE enum_boolean_defaults (
            id INT NOT NULL AUTO_INCREMENT,
            confirmed ENUM ('true', 'false', 'rumor') NOT NULL DEFAULT 'true',
            PRIMARY KEY (id)
        );
    "#})
        .await;

    let expected = expect![[r#"
        model enum_boolean_defaults {
          id        Int                             @id @default(autoincrement())
          confirmed enum_boolean_defaults_confirmed @default(true)
        }

        enum enum_boolean_defaults_confirmed {
          true
          false
          rumor
        }
    "#]];

    expected.assert_eq(&api.introspect_dml().await?);
    Ok(())
}

#[test_connector(tags(KingbaseMysql))]
async fn multiple_enums_are_stable_across_reintrospection(api: &mut TestApi) -> TestResult {
    api.raw_cmd(indoc! {r#"
        CREATE TABLE enum_catalog (
            id INT NOT NULL AUTO_INCREMENT PRIMARY KEY,
            color ENUM ('black', 'white') NOT NULL,
            color2 ENUM ('black2', 'white2') NOT NULL
        );
    "#})
        .await;

    let expected = expect![[r#"
        model enum_catalog {
          id     Int                 @id @default(autoincrement())
          color  enum_catalog_color
          color2 enum_catalog_color2
        }

        enum enum_catalog_color {
          black
          white
        }

        enum enum_catalog_color2 {
          black2
          white2
        }
    "#]];

    for _ in 0..4 {
        expected.assert_eq(&api.introspect_dml().await?);
    }

    Ok(())
}

#[test_connector(tags(KingbaseMysql))]
async fn enum_default_values_are_introspected(api: &mut TestApi) -> TestResult {
    api.raw_cmd(indoc! {r#"
        CREATE TABLE enum_defaults (
            id INT NOT NULL AUTO_INCREMENT PRIMARY KEY,
            color ENUM ('black', 'white') NOT NULL DEFAULT 'black'
        );
    "#})
        .await;

    let expected = expect![[r#"
        model enum_defaults {
          id    Int                 @id @default(autoincrement())
          color enum_defaults_color @default(black)
        }

        enum enum_defaults_color {
          black
          white
        }
    "#]];

    expected.assert_eq(&api.introspect_dml().await?);
    Ok(())
}
