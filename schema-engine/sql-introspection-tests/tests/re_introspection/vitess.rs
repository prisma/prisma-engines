use expect_test::expect;
use indoc::indoc;
use quaint::prelude::Queryable;
use sql_introspection_tests::test_api::*;
use test_macros::test_connector;

#[test_connector(tags(Vitess))]
async fn relation_mode_parameter_is_not_removed(api: &mut TestApi) -> TestResult {
    let result = api.re_introspect("").await?;
    assert!(result.contains(r#"relationMode = "prisma""#));

    Ok(())
}

#[test_connector(tags(Vitess))]
async fn relations_are_not_removed(api: &mut TestApi) -> TestResult {
    let dml = indoc! {r#"
        CREATE TABLE `A` (
            id INT AUTO_INCREMENT PRIMARY KEY
        );

        CREATE TABLE `B` (
            id  INT AUTO_INCREMENT PRIMARY KEY,
            aId INT NOT NULL
        );
    "#};

    api.database().raw_cmd(dml).await?;

    let input_dm = indoc! {r#"
        model A {
          id Int @id @default(autoincrement())
          bs B[]
        }

        model B {
          id  Int @id @default(autoincrement())
          aId Int
          a   A   @relation(fields: [aId], references: [id])
        }
    "#};

    let expected = expect![[r#"
        model A {
          id Int @id @default(autoincrement())
          bs B[]
        }

        model B {
          id  Int @id @default(autoincrement())
          aId Int
          a   A   @relation(fields: [aId], references: [id])
        }
    "#]];

    expected.assert_eq(&api.re_introspect_dml(input_dm).await?);

    Ok(())
}

#[test_connector(tags(Vitess))]
async fn warning_is_given_for_copied_relations(api: &mut TestApi) -> TestResult {
    let dml = indoc! {r#"
        CREATE TABLE `A` (
            id INT AUTO_INCREMENT PRIMARY KEY
        );

        CREATE TABLE `B` (
            id  INT AUTO_INCREMENT PRIMARY KEY,
            aId INT NOT NULL
        );
    "#};

    api.database().raw_cmd(dml).await?;

    let input_dm = indoc! {r#"
        model A {
          id Int @id @default(autoincrement())
          bs B[]
        }

        model B {
          id  Int @id @default(autoincrement())
          aId Int
          a   A   @relation(fields: [aId], references: [id])
        }
    "#};

    let expected = expect![[r#"
        *** WARNING ***

        Relations were copied from the previous data model due to not using foreign keys in the database. If any of the relation columns changed in the database, the relations might not be correct anymore:
          - "B"
          - "A"
    "#]];

    api.expect_re_introspect_warnings(input_dm, expected).await;

    Ok(())
}

#[test_connector(tags(Vitess))]
async fn no_warnings_are_given_for_if_no_relations_were_copied(api: &mut TestApi) -> TestResult {
    let dml = indoc! {r#"
        CREATE TABLE `A` (
            id INT AUTO_INCREMENT PRIMARY KEY
        );

        CREATE TABLE `B` (
            id  INT AUTO_INCREMENT PRIMARY KEY,
            aId INT NOT NULL
        );
    "#};

    api.database().raw_cmd(dml).await?;

    let input_dm = indoc! {r#"
        model A {
          id Int @id @default(autoincrement())
        }

        model B {
          id  Int @id @default(autoincrement())
          aId Int
        }
    "#};

    let expected = expect![[""]];
    api.expect_re_introspect_warnings(input_dm, expected).await;

    Ok(())
}

#[test_connector(tags(Vitess))]
async fn relations_field_order_is_kept(api: &mut TestApi) -> TestResult {
    let dml = indoc! {r#"
        CREATE TABLE `A` (
            id INT AUTO_INCREMENT PRIMARY KEY
        );

        CREATE TABLE `B` (
            id  INT AUTO_INCREMENT PRIMARY KEY,
            aId INT NOT NULL
        );
    "#};

    api.database().raw_cmd(dml).await?;

    let input_dm = indoc! {r#"
        model A {
          bs B[]
          id Int @id @default(autoincrement())
        }

        model B {
          id  Int @id @default(autoincrement())
          a   A   @relation(fields: [aId], references: [id])
          aId Int
        }
    "#};

    let expected = expect![[r#"
        model A {
          id Int @id @default(autoincrement())
          bs B[]
        }

        model B {
          id  Int @id @default(autoincrement())
          aId Int
          a   A   @relation(fields: [aId], references: [id])
        }
    "#]];

    expected.assert_eq(&api.re_introspect_dml(input_dm).await?);

    Ok(())
}

#[test_connector(tags(Vitess))]
async fn relations_field_order_is_kept_if_having_new_fields(api: &mut TestApi) -> TestResult {
    let dml = indoc! {r#"
        CREATE TABLE `A` (
            new VARCHAR(255) NOT NULL, 
            id INT AUTO_INCREMENT PRIMARY KEY
        );

        CREATE TABLE `B` (
            id  INT AUTO_INCREMENT PRIMARY KEY,
            aId INT NOT NULL
        );
    "#};

    api.database().raw_cmd(dml).await?;

    let input_dm = indoc! {r#"
        model A {
          bs B[]
          id Int @id @default(autoincrement())
        }

        model B {
          id  Int @id @default(autoincrement())
          a   A   @relation(fields: [aId], references: [id])
          aId Int
        }
    "#};

    let expected = expect![[r#"
        model A {
          new String @db.VarChar(255)
          id  Int    @id @default(autoincrement())
          bs  B[]
        }

        model B {
          id  Int @id @default(autoincrement())
          aId Int
          a   A   @relation(fields: [aId], references: [id])
        }
    "#]];

    expected.assert_eq(&api.re_introspect_dml(input_dm).await?);

    Ok(())
}

#[test_connector(tags(Vitess))]
async fn relations_field_order_is_kept_if_removing_fields(api: &mut TestApi) -> TestResult {
    let dml = indoc! {r#"
        CREATE TABLE `A` (
            id INT AUTO_INCREMENT PRIMARY KEY
        );

        CREATE TABLE `B` (
            id  INT AUTO_INCREMENT PRIMARY KEY,
            aId INT NOT NULL
        );
    "#};

    api.database().raw_cmd(dml).await?;

    let input_dm = indoc! {r#"
        model A {
          new String
          bs  B[]
          id  Int    @id @default(autoincrement())
        }

        model B {
          id  Int @id @default(autoincrement())
          a   A   @relation(fields: [aId], references: [id])
          aId Int
        }
    "#};

    let expected = expect![[r#"
        model A {
          id Int @id @default(autoincrement())
          bs B[]
        }

        model B {
          id  Int @id @default(autoincrement())
          aId Int
          a   A   @relation(fields: [aId], references: [id])
        }
    "#]];

    expected.assert_eq(&api.re_introspect_dml(input_dm).await?);

    Ok(())
}

#[test_connector(tags(Vitess))]
async fn deleting_models_will_delete_relations(api: &mut TestApi) -> TestResult {
    let dml = indoc! {r#"
        CREATE TABLE `A` (
            id INT AUTO_INCREMENT PRIMARY KEY
        );

        CREATE TABLE `B` (
            id  INT AUTO_INCREMENT PRIMARY KEY,
            aId INT NOT NULL
        );
    "#};

    api.database().raw_cmd(dml).await?;

    let input_dm = indoc! {r#"
        model A {
          id Int @id @default(autoincrement())
          bs B[]
          cs C[]
        }

        model B {
          id  Int @id @default(autoincrement())
          aId Int
          a   A   @relation(fields: [aId], references: [id])
        }

        model C {
          id  Int @id @default(autoincrement())
          aId Int
          a   A   @relation(fields: [aId], references: [id])
        }
    "#};

    let expected = expect![[r#"
        model A {
          id Int @id @default(autoincrement())
          bs B[]
        }

        model B {
          id  Int @id @default(autoincrement())
          aId Int
          a   A   @relation(fields: [aId], references: [id])
        }
    "#]];

    expected.assert_eq(&api.re_introspect_dml(input_dm).await?);

    Ok(())
}

#[test_connector(tags(Vitess))]
async fn field_renames_keeps_the_relation_intact(api: &mut TestApi) -> TestResult {
    let dml = indoc! {r#"
        CREATE TABLE `A` (
            id INT AUTO_INCREMENT PRIMARY KEY
        );

        CREATE TABLE `B` (
            id  INT AUTO_INCREMENT PRIMARY KEY,
            aId INT NOT NULL
        );
    "#};

    api.database().raw_cmd(dml).await?;

    let input_dm = indoc! {r#"
        model A {
          id Int @id @default(autoincrement())
          bs B[]
        }

        model B {
          id  Int @id @default(autoincrement())
          xId Int
          a   A   @relation(fields: [xId], references: [id])
        }
    "#};

    let expected = expect![[r#"
        model A {
          id Int @id @default(autoincrement())
          bs B[]
        }

        model B {
          id  Int @id @default(autoincrement())
          aId Int
          a   A   @relation(fields: [xId], references: [id])
        }
    "#]];

    expected.assert_eq(&api.re_introspect_dml(input_dm).await?);

    Ok(())
}

#[test_connector(tags(Vitess))]
async fn referential_actions_are_kept_intact(api: &mut TestApi) -> TestResult {
    let dml = indoc! {r#"
        CREATE TABLE `A` (
            id INT AUTO_INCREMENT PRIMARY KEY
        );

        CREATE TABLE `B` (
            id  INT AUTO_INCREMENT PRIMARY KEY,
            aId INT NULL
        );
    "#};

    api.database().raw_cmd(dml).await?;

    let input_dm = indoc! {r#"
        model A {
          id Int @id @default(autoincrement())
          bs B[]
        }

        model B {
          id  Int  @id @default(autoincrement())
          aId Int?
          a   A?   @relation(fields: [aId], references: [id], onDelete: Restrict)
        }
    "#};

    let expected = expect![[r#"
        model A {
          id Int @id @default(autoincrement())
          bs B[]
        }

        model B {
          id  Int  @id @default(autoincrement())
          aId Int?
          a   A?   @relation(fields: [aId], references: [id], onDelete: Restrict)
        }
    "#]];

    expected.assert_eq(&api.re_introspect_dml(input_dm).await?);

    Ok(())
}
