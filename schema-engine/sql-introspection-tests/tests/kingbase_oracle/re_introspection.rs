use indoc::indoc;
use sql_introspection_tests::test_api::*;
use test_macros::test_connector;

#[test_connector(tags(KingbaseOracle))]
async fn oracle_reintrospection_keeps_the_datasource_provider(api: &mut TestApi) -> TestResult {
    let schema = indoc! {r#"
        generator client {
          provider = "prisma-client"
        }

        datasource db {
          provider = "kingbase-oracle"
        }
    "#};

    let result = api.re_introspect_config(schema).await?;

    assert!(result.contains(r#"provider = "kingbase-oracle""#), "{result}");
    assert!(result.contains(r#"provider = "prisma-client""#), "{result}");

    Ok(())
}

#[test_connector(tags(KingbaseOracle))]
async fn oracle_reintrospection_preserves_maps_and_relation_names(api: &mut TestApi) -> TestResult {
    api.raw_cmd(
        r#"
        CREATE TABLE "User" (
            id INTEGER PRIMARY KEY
        );

        CREATE TABLE post (
            id INTEGER PRIMARY KEY,
            user_id INTEGER NOT NULL,
            CONSTRAINT post_user_fk FOREIGN KEY (user_id) REFERENCES "User" (id)
        );
        "#,
    )
    .await;

    let previous_schema = indoc! {r#"
        model CustomPost {
          custom_id Int        @id @map("id")
          owner_id  Int        @map("user_id")
          owner     CustomUser @relation("CustomOwner", fields: [owner_id], references: [custom_id], map: "post_user_fk")

          @@map("post")
        }

        model CustomUser {
          custom_id Int          @id @map("id")
          posts     CustomPost[] @relation("CustomOwner")

          @@map("User")
        }
    "#};

    let result = api.re_introspect_dml(previous_schema).await?;

    for expected in [
        "model CustomPost",
        "custom_id Int        @id @map(\"id\")",
        "owner_id  Int        @map(\"user_id\")",
        "@relation(\"CustomOwner\", fields: [owner_id], references: [custom_id]",
        "@@map(\"post\")",
        "model CustomUser",
        "@@map(\"User\")",
    ] {
        assert!(result.contains(expected), "missing `{expected}` in:\n{result}");
    }

    Ok(())
}

#[test_connector(tags(KingbaseOracle))]
async fn oracle_reintrospection_keeps_distinct_names_for_multiple_relations(api: &mut TestApi) -> TestResult {
    api.raw_cmd(
        r#"
        CREATE TABLE employee (
            id INTEGER PRIMARY KEY
        );

        CREATE TABLE schedule (
            id INTEGER PRIMARY KEY,
            morning_employee_id INTEGER NOT NULL,
            evening_employee_id INTEGER NOT NULL,
            CONSTRAINT schedule_morning_employee_fk
                FOREIGN KEY (morning_employee_id) REFERENCES employee (id),
            CONSTRAINT schedule_evening_employee_fk
                FOREIGN KEY (evening_employee_id) REFERENCES employee (id)
        );
        "#,
    )
    .await;

    let previous_schema = indoc! {r#"
        model Employee {
          id              Int        @id
          morningSchedules Schedule[] @relation("MorningEmployee")
          eveningSchedules Schedule[] @relation("EveningEmployee")

          @@map("employee")
        }

        model Schedule {
          id                Int      @id
          morningEmployeeId Int      @map("morning_employee_id")
          eveningEmployeeId Int      @map("evening_employee_id")
          morningEmployee   Employee @relation("MorningEmployee", fields: [morningEmployeeId], references: [id], map: "schedule_morning_employee_fk")
          eveningEmployee   Employee @relation("EveningEmployee", fields: [eveningEmployeeId], references: [id], map: "schedule_evening_employee_fk")

          @@map("schedule")
        }
    "#};

    let result = api.re_introspect_dml(previous_schema).await?;

    for expected in [
        "morningSchedules Schedule[] @relation(\"MorningEmployee\")",
        "eveningSchedules Schedule[] @relation(\"EveningEmployee\")",
        "@relation(\"MorningEmployee\", fields: [morningEmployeeId], references: [id]",
        "@relation(\"EveningEmployee\", fields: [eveningEmployeeId], references: [id]",
    ] {
        assert!(result.contains(expected), "missing `{expected}` in:\n{result}");
    }

    Ok(())
}

#[test_connector(tags(KingbaseOracle))]
async fn oracle_reintrospection_does_not_add_relation_mode(api: &mut TestApi) -> TestResult {
    let result = api.re_introspect("").await?;

    assert!(!result.contains(r#"relationMode = "#), "{result}");

    Ok(())
}

#[test_connector(tags(KingbaseOracle))]
async fn oracle_reintrospection_keeps_empty_preview_features(api: &mut TestApi) -> TestResult {
    let schema = indoc! {r#"
        generator client {
          provider = "prisma-client"
        }

        datasource db {
          provider = "kingbase-oracle"
        }
    "#};

    let result = api.re_introspect_config(schema).await?;

    assert!(result.contains("provider = \"prisma-client\""), "{result}");
    assert!(result.contains("provider = \"kingbase-oracle\""), "{result}");
    assert!(!result.contains("previewFeatures"), "{result}");

    Ok(())
}
