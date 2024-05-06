use barrel::types;
use sql_introspection_tests::test_api::*;

fn with_config(dm: &str, config: String) -> String {
    format!("{config}\n{dm}")
}

// ----- Models -----

#[test_connector(exclude(CockroachDb))]
async fn reintrospect_new_model_single_file(api: &mut TestApi) -> TestResult {
    api.barrel()
        .execute(|migration| {
            migration.create_table("User", |t| {
                t.add_column("id", types::integer().increments(true));
                t.add_constraint("User_pkey", types::primary_constraint(vec!["id"]));
            });

            migration.create_table("Unrelated", |t| {
                t.add_column("id", types::integer().increments(true));
                t.add_constraint("Unrelated_pkey", types::primary_constraint(vec!["id"]));
            });
        })
        .await?;

    let config = &api.pure_config();
    let main_dm = indoc! {r#"
      model User {
          id Int @id @default(autoincrement())
      }
    "#};

    let input_dms = [("main.prisma", format!("{config}\n{main_dm}",))];

    let expected = expect![[r#"
        // file: main.prisma
        model User {
          id Int @id @default(autoincrement())
        }

        model Unrelated {
          id Int @id @default(autoincrement())
        }
    "#]];

    api.expect_re_introspected_datamodels(&input_dms, expected).await;

    api.expect_no_warnings().await;

    Ok(())
}

#[test_connector(exclude(CockroachDb))]
async fn reintrospect_new_model_multi_file(api: &mut TestApi) -> TestResult {
    api.barrel()
        .execute(|migration| {
            migration.create_table("User", |t| {
                t.add_column("id", types::integer().increments(true));
                t.add_constraint("User_pkey", types::primary_constraint(vec!["id"]));
            });

            migration.create_table("Post", |t| {
                t.add_column("id", types::integer().increments(true));
                t.add_constraint("Post_pkey", types::primary_constraint(vec!["id"]));
            });

            migration.create_table("Unrelated", |t| {
                t.add_column("id", types::integer().increments(true));
                t.add_constraint("Unrelated_pkey", types::primary_constraint(vec!["id"]));
            });
        })
        .await?;

    let user_dm = indoc! {r#"
      model User {
          id Int @id @default(autoincrement())
      }
    "#};
    let post_dm = indoc! {r#"
      model Post {
        id Int @id @default(autoincrement())
      }
    "#};

    let input_dms = [
        ("user.prisma", with_config(user_dm, api.pure_config())),
        ("post.prisma", post_dm.to_string()),
    ];

    let expected = expect![[r#"
        // file: introspected.prisma
        model Unrelated {
          id Int @id @default(autoincrement())
        }
        ------
        // file: post.prisma
        model Post {
          id Int @id @default(autoincrement())
        }
        ------
        // file: user.prisma
        model User {
          id Int @id @default(autoincrement())
        }
    "#]];

    api.expect_re_introspected_datamodels(&input_dms, expected).await;

    Ok(())
}

#[test_connector(exclude(CockroachDb))]
async fn reintrospect_removed_model_single_file(api: &mut TestApi) -> TestResult {
    api.barrel()
        .execute(|migration| {
            migration.create_table("User", |t| {
                t.add_column("id", types::integer().increments(true));
                t.add_constraint("User_pkey", types::primary_constraint(vec!["id"]));
            });
        })
        .await?;

    let config = &api.pure_config();
    let main_dm = indoc! {r#"
      model User {
          id Int @id @default(autoincrement())
      }

      model Removed {
        id Int @id @default(autoincrement())
      }
    "#};

    let input_dms = [("main.prisma", format!("{config}\n{main_dm}",))];

    let expected = expect![[r#"
        // file: main.prisma
        model User {
          id Int @id @default(autoincrement())
        }
    "#]];

    api.expect_re_introspected_datamodels(&input_dms, expected).await;

    api.expect_no_warnings().await;

    Ok(())
}

#[test_connector(exclude(CockroachDb))]
async fn reintrospect_removed_model_multi_file(api: &mut TestApi) -> TestResult {
    api.barrel()
        .execute(|migration| {
            migration.create_table("User", |t| {
                t.add_column("id", types::integer().increments(true));
                t.add_constraint("User_pkey", types::primary_constraint(vec!["id"]));
            });

            migration.create_table("Unrelated", |t| {
                t.add_column("id", types::integer().increments(true));
                t.add_constraint("Unrelated_pkey", types::primary_constraint(vec!["id"]));
            });
        })
        .await?;

    let user_dm = indoc! {r#"
      model User {
          id Int @id @default(autoincrement())
      }
    "#};
    let post_dm = indoc! {r#"
      model Post {
        id Int @id @default(autoincrement())
      }
    "#};

    let input_dms = [
        ("user.prisma", with_config(user_dm, api.pure_config())),
        ("post.prisma", post_dm.to_string()),
    ];

    let expected = expect![[r#"
        // file: introspected.prisma
        model Unrelated {
          id Int @id @default(autoincrement())
        }
        ------
        // file: post.prisma

        ------
        // file: user.prisma
        model User {
          id Int @id @default(autoincrement())
        }
    "#]];

    api.expect_re_introspected_datamodels(&input_dms, expected).await;

    Ok(())
}

// ----- Enums -----

#[test_connector(tags(Postgres), exclude(CockroachDb))]
async fn reintrospect_new_enum_single_file(api: &mut TestApi) -> TestResult {
    api.barrel()
        .execute(|migration| {
            migration.create_table("User", |t| {
                t.add_column("id", types::integer().increments(true));
                t.add_constraint("User_pkey", types::primary_constraint(vec!["id"]));
            });
        })
        .await?;

    api.raw_cmd(r#"CREATE TYPE "theEnumName" AS ENUM ('A', 'B');"#).await;

    let main_dm = indoc! {r#"
      model User {
          id Int @id @default(autoincrement())
      }
    "#};

    let input_dms = [("main.prisma", with_config(main_dm, api.pure_config()))];

    let expected = expect![[r#"
        // file: main.prisma
        model User {
          id Int @id @default(autoincrement())
        }

        enum theEnumName {
          A
          B
        }
    "#]];

    api.expect_re_introspected_datamodels(&input_dms, expected).await;

    api.expect_no_warnings().await;

    Ok(())
}

#[test_connector(tags(Postgres), exclude(CockroachDb))]
async fn reintrospect_removed_enum_single_file(api: &mut TestApi) -> TestResult {
    api.barrel()
        .execute(|migration| {
            migration.create_table("User", |t| {
                t.add_column("id", types::integer().increments(true));
                t.add_constraint("User_pkey", types::primary_constraint(vec!["id"]));
            });
        })
        .await?;

    let main_dm = indoc! {r#"
      model User {
          id Int @id @default(autoincrement())
      }

      enum removedEnum {
        A
        B
      }
    "#};

    let input_dms = [("main.prisma", with_config(main_dm, api.pure_config()))];

    let expected = expect![[r#"
        // file: main.prisma
        model User {
          id Int @id @default(autoincrement())
        }
    "#]];

    api.expect_re_introspected_datamodels(&input_dms, expected).await;

    api.expect_no_warnings().await;

    Ok(())
}

#[test_connector(tags(Postgres), exclude(CockroachDb))]
async fn reintrospect_new_enum_multi_file(api: &mut TestApi) -> TestResult {
    api.barrel()
        .execute(|migration| {
            migration.create_table("User", |t| {
                t.add_column("id", types::integer().increments(true));
                t.add_constraint("User_pkey", types::primary_constraint(vec!["id"]));
            });

            migration.create_table("Post", |t| {
                t.add_column("id", types::integer().increments(true));
                t.add_constraint("Post_pkey", types::primary_constraint(vec!["id"]));
            });
        })
        .await?;

    api.raw_cmd(r#"CREATE TYPE "theEnumName" AS ENUM ('A', 'B');"#).await;

    let config = &api.pure_config();
    let user_dm = indoc! {r#"
      model User {
          id Int @id @default(autoincrement())
      }
    "#};
    let post_dm = indoc! {r#"
      model Post {
        id Int @id @default(autoincrement())
      }
    "#};

    let input_dms = [
        ("user.prisma", format!("{config}\n{user_dm}")),
        ("post.prisma", post_dm.to_string()),
    ];

    let expected = expect![[r#"
        // file: introspected.prisma
        enum theEnumName {
          A
          B
        }
        ------
        // file: post.prisma
        model Post {
          id Int @id @default(autoincrement())
        }
        ------
        // file: user.prisma
        model User {
          id Int @id @default(autoincrement())
        }
    "#]];

    api.expect_re_introspected_datamodels(&input_dms, expected).await;

    api.expect_no_warnings().await;

    Ok(())
}

#[test_connector(tags(Postgres), exclude(CockroachDb))]
async fn reintrospect_removed_enum_multi_file(api: &mut TestApi) -> TestResult {
    api.barrel()
        .execute(|migration| {
            migration.create_table("User", |t| {
                t.add_column("id", types::integer().increments(true));
                t.add_constraint("User_pkey", types::primary_constraint(vec!["id"]));
            });
        })
        .await?;

    let config = &api.pure_config();
    let user_dm = indoc! {r#"
      model User {
          id Int @id @default(autoincrement())
      }
    "#};
    let enum_dm = indoc! {r#"
      enum theEnumName {
        A
        B
      }
    "#};

    let input_dms = [
        ("user.prisma", format!("{config}\n{user_dm}")),
        ("enum.prisma", enum_dm.to_string()),
    ];

    let expected = expect![[r#"
        // file: enum.prisma

        ------
        // file: user.prisma
        model User {
          id Int @id @default(autoincrement())
        }
    "#]];

    api.expect_re_introspected_datamodels(&input_dms, expected).await;

    api.expect_no_warnings().await;

    Ok(())
}

// ----- Views -----

#[test_connector(tags(Postgres), exclude(CockroachDb))]
async fn introspect_multi_view_preview_feature_is_required(api: &mut TestApi) -> TestResult {
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
        // file: schema.prisma
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

    api.expect_datamodels(&expected).await;

    api.expect_no_warnings().await;

    Ok(())
}

#[test_connector(tags(Postgres), exclude(Postgres16), exclude(CockroachDb), preview_features("views"))]
// the expect_view_definition is slightly different than for Postgres16
async fn reintrospect_new_view_single_file(api: &mut TestApi) -> TestResult {
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

    let main_dm = with_config(
        indoc! {r#"
        model User {
            id Int @id @default(autoincrement())
        }
      "#},
        api.pure_config(),
    );
    let input_dms = [("main.prisma", main_dm)];

    let expected = expect![[r#"
        // file: main.prisma
        model User {
          id         Int     @id @default(autoincrement())
          first_name String  @db.VarChar(255)
          last_name  String? @db.VarChar(255)
        }

        /// The underlying view does not contain a valid unique identifier and can therefore currently not be handled by Prisma Client.
        view Schwuser {
          id         Int?
          first_name String? @db.VarChar(255)
          last_name  String? @db.VarChar(255)

          @@ignore
        }
    "#]];

    api.expect_re_introspected_datamodels(&input_dms, expected).await;

    let expected = expect![[r#"
        SELECT
          "User".id,
          "User".first_name,
          "User".last_name
        FROM
          "User";"#]];

    api.expect_view_definition("Schwuser", &expected).await;

    let expected = expect![[r#"
        *** WARNING ***

        The following views were ignored as they do not have a valid unique identifier or id. This is currently not supported by Prisma Client. Please refer to the documentation on defining unique identifiers in views: https://pris.ly/d/view-identifiers
          - "Schwuser"
    "#]];
    api.expect_warnings(&expected).await;

    Ok(())
}

#[test_connector(tags(Postgres), exclude(Postgres16), exclude(CockroachDb), preview_features("views"))]
// the expect_view_definition is slightly different than for Postgres16
async fn reintrospect_removed_view_single_file(api: &mut TestApi) -> TestResult {
    let setup = indoc! {r#"
        CREATE TABLE "User" (
            id SERIAL PRIMARY KEY,
            first_name VARCHAR(255) NOT NULL,
            last_name VARCHAR(255) NULL
        );
    "#};

    api.raw_cmd(setup).await;

    let main_dm = with_config(
        indoc! {r#"
        model User {
            id Int @id @default(autoincrement())
        }

        view RemovedView {
          id         Int?
          first_name String? @db.VarChar(255)
          last_name  String? @db.VarChar(255)

          @@ignore
        }
      "#},
        api.pure_config(),
    );
    let input_dms = [("main.prisma", main_dm)];

    let expected = expect![[r#"
        // file: main.prisma
        model User {
          id         Int     @id @default(autoincrement())
          first_name String  @db.VarChar(255)
          last_name  String? @db.VarChar(255)
        }
    "#]];

    api.expect_re_introspected_datamodels(&input_dms, expected).await;

    let expected = expect![""];
    api.expect_warnings(&expected).await;

    Ok(())
}

#[test_connector(tags(Postgres), exclude(Postgres16), exclude(CockroachDb), preview_features("views"))]
// the expect_view_definition is slightly different than for Postgres16
async fn reintrospect_new_view_multi_file(api: &mut TestApi) -> TestResult {
    let setup = indoc! {r#"
        CREATE TABLE "User" (
            id SERIAL PRIMARY KEY,
            first_name VARCHAR(255) NOT NULL,
            last_name VARCHAR(255) NULL
        );

        CREATE TABLE "Post" (
          id SERIAL PRIMARY KEY
      );

        CREATE VIEW "Schwuser" AS
            SELECT id, first_name, last_name FROM "User";
    "#};

    api.raw_cmd(setup).await;

    let user_dm = with_config(
        indoc! {r#"
        model User {
            id Int @id @default(autoincrement())
        }
      "#},
        api.pure_config(),
    );
    let post_dm = indoc! {r#"
      model Post {
        id Int @id @default(autoincrement())
      }
    "#};
    let input_dms = [("user.prisma", user_dm), ("post.prisma", post_dm.to_string())];

    let expected = expect![[r#"
        // file: introspected.prisma
        /// The underlying view does not contain a valid unique identifier and can therefore currently not be handled by Prisma Client.
        view Schwuser {
          id         Int?
          first_name String? @db.VarChar(255)
          last_name  String? @db.VarChar(255)

          @@ignore
        }
        ------
        // file: post.prisma
        model Post {
          id Int @id @default(autoincrement())
        }
        ------
        // file: user.prisma
        model User {
          id         Int     @id @default(autoincrement())
          first_name String  @db.VarChar(255)
          last_name  String? @db.VarChar(255)
        }
    "#]];

    api.expect_re_introspected_datamodels(&input_dms, expected).await;

    let expected = expect![[r#"
        SELECT
          "User".id,
          "User".first_name,
          "User".last_name
        FROM
          "User";"#]];

    api.expect_view_definition_multi("Schwuser", &expected).await;

    let expected = expect![[r#"
        *** WARNING ***

        The following views were ignored as they do not have a valid unique identifier or id. This is currently not supported by Prisma Client. Please refer to the documentation on defining unique identifiers in views: https://pris.ly/d/view-identifiers
          - "Schwuser"
    "#]];
    api.expect_warnings(&expected).await;

    Ok(())
}

#[test_connector(tags(Postgres), exclude(Postgres16), exclude(CockroachDb), preview_features("views"))]
// the expect_view_definition is slightly different than for Postgres16
async fn reintrospect_removed_view_multi_file(api: &mut TestApi) -> TestResult {
    let setup = indoc! {r#"
        CREATE TABLE "User" (
            id SERIAL PRIMARY KEY,
            first_name VARCHAR(255) NOT NULL,
            last_name VARCHAR(255) NULL
        );
    "#};

    api.raw_cmd(setup).await;

    let user_dm = with_config(
        indoc! {r#"
          model User {
              id Int @id @default(autoincrement())
          }
        "#},
        api.pure_config(),
    );
    let view_dm = indoc! {r#"
      view Schwuser {
        id         Int?
        first_name String? @db.VarChar(255)
        last_name  String? @db.VarChar(255)

        @@ignore
      }
    "#};
    let input_dms = [("user.prisma", user_dm), ("view.prisma", view_dm.to_string())];

    let expected = expect![[r#"
        // file: user.prisma
        model User {
          id         Int     @id @default(autoincrement())
          first_name String  @db.VarChar(255)
          last_name  String? @db.VarChar(255)
        }
        ------
        // file: view.prisma

    "#]];

    api.expect_re_introspected_datamodels(&input_dms, expected).await;

    let expected = expect![""];
    api.expect_warnings(&expected).await;

    Ok(())
}

// ----- Configuration -----
#[test_connector(tags(Postgres), exclude(CockroachDb))]
async fn reintrospect_keep_configuration_in_same_file(api: &mut TestApi) -> TestResult {
    api.barrel()
        .execute(|migration| {
            migration.create_table("User", |t| {
                t.add_column("id", types::integer().increments(true));
                t.add_constraint("User_pkey", types::primary_constraint(vec!["id"]));
            });

            migration.create_table("Post", |t| {
                t.add_column("id", types::integer().increments(true));
                t.add_constraint("Post_pkey", types::primary_constraint(vec!["id"]));
            });
        })
        .await?;

    let user_dm = indoc! {r#"
      model User {
          id Int @id @default(autoincrement())
      }
    "#};
    let post_dm = indoc! {r#"
      model Post {
        id Int @id @default(autoincrement())
      }
    "#};

    let expected = expect![[r#"
        // file: post.prisma
        model Post {
          id Int @id @default(autoincrement())
        }
        ------
        // file: user.prisma
        generator client {
          provider = "prisma-client-js"
        }

        datasource db {
          provider = "postgresql"
          url      = "env(TEST_DATABASE_URL)"
        }

        model User {
          id Int @id @default(autoincrement())
        }
    "#]];

    api.expect_re_introspected_datamodels_with_config(
        &[
            ("user.prisma", with_config(user_dm, api.pure_config())),
            ("post.prisma", post_dm.to_string()),
        ],
        expected,
    )
    .await;

    let expected = expect![[r#"
        // file: post.prisma
        generator client {
          provider = "prisma-client-js"
        }

        datasource db {
          provider = "postgresql"
          url      = "env(TEST_DATABASE_URL)"
        }

        model Post {
          id Int @id @default(autoincrement())
        }
        ------
        // file: user.prisma
        model User {
          id Int @id @default(autoincrement())
        }
    "#]];

    api.expect_re_introspected_datamodels_with_config(
        &[
            ("user.prisma", user_dm.to_string()),
            ("post.prisma", with_config(post_dm, api.pure_config())),
        ],
        expected,
    )
    .await;

    Ok(())
}

#[test_connector(tags(Postgres), exclude(CockroachDb))]
async fn reintrospect_keep_configuration_when_spread_across_files(api: &mut TestApi) -> TestResult {
    api.barrel()
        .execute(|migration| {
            migration.create_table("User", |t| {
                t.add_column("id", types::integer().increments(true));
                t.add_constraint("User_pkey", types::primary_constraint(vec!["id"]));
            });

            migration.create_table("Post", |t| {
                t.add_column("id", types::integer().increments(true));
                t.add_constraint("Post_pkey", types::primary_constraint(vec!["id"]));
            });
        })
        .await?;

    let user_dm = indoc! {r#"
      model User {
          id Int @id @default(autoincrement())
      }
    "#};
    let post_dm = indoc! {r#"
      model Post {
        id Int @id @default(autoincrement())
      }
    "#};

    let expected = expect![[r#"
        // file: post.prisma
        generator client {
          provider = "prisma-client-js"
        }

        model Post {
          id Int @id @default(autoincrement())
        }
        ------
        // file: user.prisma
        datasource db {
          provider = "postgresql"
          url      = "env(TEST_DATABASE_URL)"
        }

        model User {
          id Int @id @default(autoincrement())
        }
    "#]];

    api.expect_re_introspected_datamodels_with_config(
        &[
            ("user.prisma", format!("{}\n{user_dm}", api.datasource_block_string())),
            ("post.prisma", format!("{}\n{post_dm}", api.generator_block_string())),
        ],
        expected,
    )
    .await;

    let expected = expect![[r#"
        // file: post.prisma
        datasource db {
          provider = "postgresql"
          url      = "env(TEST_DATABASE_URL)"
        }

        model Post {
          id Int @id @default(autoincrement())
        }
        ------
        // file: user.prisma
        generator client {
          provider = "prisma-client-js"
        }

        model User {
          id Int @id @default(autoincrement())
        }
    "#]];

    api.expect_re_introspected_datamodels_with_config(
        &[
            ("user.prisma", format!("{}\n{user_dm}", api.generator_block_string())),
            ("post.prisma", format!("{}\n{post_dm}", api.datasource_block_string())),
        ],
        expected,
    )
    .await;

    Ok(())
}

#[test_connector(tags(Postgres), exclude(CockroachDb))]
async fn reintrospect_keep_configuration_when_no_models(api: &mut TestApi) -> TestResult {
    api.barrel()
        .execute(|migration| {
            migration.create_table("User", |t| {
                t.add_column("id", types::integer().increments(true));
                t.add_constraint("User_pkey", types::primary_constraint(vec!["id"]));
            });
        })
        .await?;

    let user_dm = indoc! {r#"
      model User {
          id Int @id @default(autoincrement())
      }
    "#};
    let post_dm = indoc! {r#"
      model Post {
        id Int @id @default(autoincrement())
      }
    "#};

    let expected = expect![[r#"
        // file: post.prisma
        generator client {
          provider = "prisma-client-js"
        }
        ------
        // file: user.prisma
        datasource db {
          provider = "postgresql"
          url      = "env(TEST_DATABASE_URL)"
        }

        model User {
          id Int @id @default(autoincrement())
        }
    "#]];

    api.expect_re_introspected_datamodels_with_config(
        &[
            ("user.prisma", format!("{}\n{user_dm}", api.datasource_block_string())),
            ("post.prisma", format!("{}\n{post_dm}", api.generator_block_string())),
        ],
        expected,
    )
    .await;

    let expected = expect![[r#"
        // file: post.prisma
        datasource db {
          provider = "postgresql"
          url      = "env(TEST_DATABASE_URL)"
        }
        ------
        // file: user.prisma
        generator client {
          provider = "prisma-client-js"
        }

        model User {
          id Int @id @default(autoincrement())
        }
    "#]];

    api.expect_re_introspected_datamodels_with_config(
        &[
            ("user.prisma", format!("{}\n{user_dm}", api.generator_block_string())),
            ("post.prisma", format!("{}\n{post_dm}", api.datasource_block_string())),
        ],
        expected,
    )
    .await;

    Ok(())
}

// ----- Miscellaneous -----

#[test_connector(tags(Postgres), exclude(CockroachDb))]
async fn reintrospect_empty_multi_file(api: &mut TestApi) -> TestResult {
    let user_dm = indoc! {r#"
      model User {
          id Int @id @default(autoincrement())
      }
    "#};
    let post_dm = indoc! {r#"
      model Post {
        id Int @id @default(autoincrement())
      }
    "#};

    let input_dms = [
        ("user.prisma", with_config(user_dm, api.pure_config())),
        ("post.prisma", post_dm.to_string()),
    ];

    let expected = expect![[r#"
        // file: post.prisma

        ------
        // file: user.prisma
        generator client {
          provider = "prisma-client-js"
        }

        datasource db {
          provider = "postgresql"
          url      = "env(TEST_DATABASE_URL)"
        }
    "#]];

    api.expect_re_introspected_datamodels_with_config(&input_dms, expected)
        .await;

    Ok(())
}
