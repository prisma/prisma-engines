use sql_introspection_tests::test_api::*;

#[test_connector(tags(Sqlite))]
async fn an_enum_in_the_model_is_preserved_when_introspected(api: &mut TestApi) -> TestResult {
    let original = indoc! { r#"
        /// User model.
        model User {
          id   String @id @default(uuid())
          role Role
        }

        /// Role enum.
        enum Role {
          USER
          ADMIN
        }
    "#
    };

    api.raw_cmd(
        r#"
        CREATE TABLE "User" (
            "id" TEXT NOT NULL PRIMARY KEY,
            "role" TEXT NOT NULL
        );
        "#,
    )
    .await;

    let result = api.re_introspect(original).await?;
    api.assert_eq_datamodels(original, &result);
    Ok(())
}

#[test_connector(tags(Sqlite))]
async fn an_enum_in_the_model_is_preserved_without_redundant_attributes_when_introspected(
    api: &mut TestApi,
) -> TestResult {
    let original = indoc! { r#"
        /// User model.
        model User {
          id   String @id @default(uuid())
          role Role
        }

        /// Role enum.
        enum Role {
          USER
          ADMIN
          // the @map has no effect because the enum does exist in the database
          @@map("r0le")
        }
    "#
    };

    let expected = indoc! { r#"
        /// User model.
        model User {
          id   String @id @default(uuid())
          role Role
        }

        /// Role enum.
        enum Role {
          USER
          ADMIN
        }
    "#
    };

    api.raw_cmd(
        r#"
        CREATE TABLE "User" (
            "id" TEXT NOT NULL PRIMARY KEY,
            "role" TEXT NOT NULL
        );
        "#,
    )
    .await;

    let result = api.re_introspect(original).await?;
    api.assert_eq_datamodels(expected, &result);
    Ok(())
}
