use barrel::types;
use indoc::indoc;
use schema_core::{ExtensionType, ExtensionTypeConfig};
use sql_introspection_tests::test_api::*;
use test_macros::test_connector;

#[test_connector(tags(Postgres), exclude(CockroachDb), preview_features("postgresqlExtensions"))]
async fn should_work_with_the_preview_feature_enabled(api: &mut TestApi) -> TestResult {
    let setup = indoc! {r#"
        CREATE EXTENSION IF NOT EXISTS citext;
    "#};

    api.raw_cmd(setup).await;

    let expectation = expect![[r#"
        generator client {
          provider        = "prisma-client"
          previewFeatures = ["postgresqlExtensions"]
        }

        datasource db {
          provider   = "postgresql"
          url        = "dummy-url"
          extensions = [citext(schema: "public")]
        }
    "#]];

    api.expect_datamodel(&expectation).await;

    Ok(())
}

#[test_connector(tags(Postgres), exclude(CockroachDb), preview_features("postgresqlExtensions"))]
async fn sanitizes_problematic_extension_names(api: &mut TestApi) -> TestResult {
    let setup = indoc! {r#"
        CREATE EXTENSION IF NOT EXISTS "uuid-ossp";
    "#};

    api.raw_cmd(setup).await;

    let expectation = expect![[r#"
        generator client {
          provider        = "prisma-client"
          previewFeatures = ["postgresqlExtensions"]
        }

        datasource db {
          provider   = "postgresql"
          url        = "dummy-url"
          extensions = [uuid_ossp(map: "uuid-ossp", schema: "public")]
        }
    "#]];

    api.expect_datamodel(&expectation).await;

    Ok(())
}

#[test_connector(
    tags(Postgres),
    exclude(CockroachDb, Postgres9),
    preview_features("postgresqlExtensions")
)]
async fn should_not_list_any_extensions_outside_of_allow_list(api: &mut TestApi) -> TestResult {
    let setup = indoc! {r#"
        CREATE EXTENSION IF NOT EXISTS amcheck;
    "#};

    api.raw_cmd(setup).await;

    let expectation = expect![[r#"
        generator client {
          provider        = "prisma-client"
          previewFeatures = ["postgresqlExtensions"]
        }

        datasource db {
          provider = "postgresql"
          url      = "dummy-url"
        }
    "#]];

    api.expect_datamodel(&expectation).await;

    Ok(())
}

#[test_connector(
    tags(Postgres),
    exclude(CockroachDb, Postgres9),
    preview_features("postgresqlExtensions")
)]
async fn should_not_remove_any_extensions_outside_of_allow_list(api: &mut TestApi) -> TestResult {
    let setup = indoc! {r#"
        CREATE EXTENSION IF NOT EXISTS amcheck;
    "#};

    api.raw_cmd(setup).await;

    let schema = indoc! {r#"
        generator client {
          provider        = "prisma-client"
          previewFeatures = ["postgresqlExtensions"]
        }

        datasource db {
          provider   = "postgresql"
          url        = "dummy-url"
          extensions = [amcheck]
        }
    "#};

    let expectation = expect![[r#"
        generator client {
          provider        = "prisma-client"
          previewFeatures = ["postgresqlExtensions"]
        }

        datasource db {
          provider   = "postgresql"
          url        = "dummy-url"
          extensions = [amcheck]
        }
    "#]];

    expectation.assert_eq(&api.re_introspect_config(schema).await?);

    Ok(())
}

#[test_connector(tags(Postgres), exclude(CockroachDb))]
async fn should_not_list_extensions_without_the_preview_feature(api: &mut TestApi) -> TestResult {
    let setup = indoc! {r#"
        CREATE EXTENSION IF NOT EXISTS citext;
    "#};

    api.raw_cmd(setup).await;

    let expectation = expect![[r#"
        generator client {
          provider = "prisma-client"
        }

        datasource db {
          provider = "postgresql"
          url      = "dummy-url"
        }
    "#]];

    api.expect_datamodel(&expectation).await;

    Ok(())
}

#[test_connector(tags(Postgres14), exclude(CockroachDb), preview_features("postgresqlExtensions"))]
async fn should_keep_version_attribute_if_same_as_db(api: &mut TestApi) -> TestResult {
    let setup = indoc! {r#"
        CREATE EXTENSION IF NOT EXISTS citext;
    "#};

    api.raw_cmd(setup).await;

    let schema = indoc! {r#"
        generator client {
          provider        = "prisma-client"
          previewFeatures = ["postgresqlExtensions"]
        }

        datasource db {
          provider   = "postgresql"
          url        = "dummy-url"
          extensions = [citext(version: "1.6")]
        }
    "#};

    let expectation = expect![[r#"
        generator client {
          provider        = "prisma-client"
          previewFeatures = ["postgresqlExtensions"]
        }

        datasource db {
          provider   = "postgresql"
          url        = "dummy-url"
          extensions = [citext(version: "1.6")]
        }
    "#]];

    expectation.assert_eq(&api.re_introspect_config(schema).await?);

    Ok(())
}

#[test_connector(tags(Postgres14), exclude(CockroachDb), preview_features("postgresqlExtensions"))]
async fn should_update_version_attribute_if_different_than_db(api: &mut TestApi) -> TestResult {
    let setup = indoc! {r#"
        CREATE EXTENSION IF NOT EXISTS citext;
    "#};

    api.raw_cmd(setup).await;

    let schema = indoc! {r#"
        generator client {
          provider        = "prisma-client"
          previewFeatures = ["postgresqlExtensions"]
        }

        datasource db {
          provider   = "postgresql"
          url        = "dummy-url"
          extensions = [citext(version: "1.4")]
        }
    "#};

    let expectation = expect![[r#"
        generator client {
          provider        = "prisma-client"
          previewFeatures = ["postgresqlExtensions"]
        }

        datasource db {
          provider   = "postgresql"
          url        = "dummy-url"
          extensions = [citext(version: "1.6")]
        }
    "#]];

    expectation.assert_eq(&api.re_introspect_config(schema).await?);

    Ok(())
}

#[test_connector(tags(Postgres), exclude(CockroachDb), preview_features("postgresqlExtensions"))]
async fn should_keep_schema_attribute_if_same_as_db(api: &mut TestApi) -> TestResult {
    let setup = indoc! {r#"
        CREATE EXTENSION IF NOT EXISTS citext;
    "#};

    api.raw_cmd(setup).await;

    let schema = indoc! {r#"
        generator client {
          provider        = "prisma-client"
          previewFeatures = ["postgresqlExtensions"]
        }

        datasource db {
          provider   = "postgresql"
          url        = "dummy-url"
          extensions = [citext(schema: "public")]
        }
    "#};

    let expectation = expect![[r#"
        generator client {
          provider        = "prisma-client"
          previewFeatures = ["postgresqlExtensions"]
        }

        datasource db {
          provider   = "postgresql"
          url        = "dummy-url"
          extensions = [citext(schema: "public")]
        }
    "#]];

    expectation.assert_eq(&api.re_introspect_config(schema).await?);

    Ok(())
}

#[test_connector(tags(Postgres), exclude(CockroachDb), preview_features("postgresqlExtensions"))]
async fn should_update_schema_attribute_if_different_than_db(api: &mut TestApi) -> TestResult {
    let setup = indoc! {r#"
        CREATE EXTENSION IF NOT EXISTS citext;
    "#};

    api.raw_cmd(setup).await;

    let schema = indoc! {r#"
        generator client {
          provider        = "prisma-client"
          previewFeatures = ["postgresqlExtensions"]
        }

        datasource db {
          provider   = "postgresql"
          url        = "dummy-url"
          extensions = [citext(schema: "meow")]
        }
    "#};

    let expectation = expect![[r#"
        generator client {
          provider        = "prisma-client"
          previewFeatures = ["postgresqlExtensions"]
        }

        datasource db {
          provider   = "postgresql"
          url        = "dummy-url"
          extensions = [citext(schema: "public")]
        }
    "#]];

    expectation.assert_eq(&api.re_introspect_config(schema).await?);

    Ok(())
}

#[test_connector(tags(Postgres), exclude(CockroachDb), preview_features("postgresqlExtensions"))]
async fn should_remove_missing_extensions(api: &mut TestApi) -> TestResult {
    let schema = indoc! {r#"
        generator client {
          provider        = "prisma-client"
          previewFeatures = ["postgresqlExtensions"]
        }

        datasource db {
          provider   = "postgresql"
          url        = "dummy-url"
          extensions = [citext]
        }
    "#};

    let expectation = expect![[r#"
        generator client {
          provider        = "prisma-client"
          previewFeatures = ["postgresqlExtensions"]
        }

        datasource db {
          provider = "postgresql"
          url      = "dummy-url"
        }
    "#]];

    expectation.assert_eq(&api.re_introspect_config(schema).await?);

    Ok(())
}

#[test_connector(tags(Postgres), exclude(CockroachDb), preview_features("postgresqlExtensions"))]
async fn no_extensions_means_no_extensions(api: &mut TestApi) -> TestResult {
    let expectation = expect![[r#"
        generator client {
          provider        = "prisma-client"
          previewFeatures = ["postgresqlExtensions"]
        }

        datasource db {
          provider = "postgresql"
          url      = "dummy-url"
        }
    "#]];

    api.expect_datamodel(&expectation).await;

    Ok(())
}

#[test_connector(tags(Postgres), exclude(CockroachDb))]
async fn introspect_extension_type(api: &mut TestApi) -> TestResult {
    api.barrel()
        .execute(|migration| {
            migration.inject_custom("CREATE EXTENSION IF NOT EXISTS vector;");

            migration.create_table("A", |t| {
                t.add_column("id", types::primary());
                t.add_column("data", types::custom("vector(3)").nullable(false));
            });
        })
        .await?;

    let expectation = expect![[r#"
        generator client {
          provider = "prisma-client"
        }

        datasource db {
          provider = "postgresql"
          url      = "dummy-url"
        }

        model A {
          id   Int     @id @default(autoincrement())
          data Vector3
        }
    "#]];

    let extensions = ExtensionTypeConfig::new(vec![
        ExtensionType::builder()
            .prisma_name("Vector3")
            .db_name("vector")
            .db_type_modifiers(vec!["3".into()])
            .number_of_db_type_modifiers(1)
            .build(),
    ]);

    expectation.assert_eq(&api.introspect_with_extensions(&extensions).await?);

    Ok(())
}

#[test_connector(tags(Postgres), exclude(CockroachDb))]
async fn introspect_specific_extension_type_by_type_modifier(api: &mut TestApi) -> TestResult {
    api.barrel()
        .execute(|migration| {
            migration.inject_custom("CREATE EXTENSION IF NOT EXISTS vector;");

            migration.create_table("A", |t| {
                t.add_column("id", types::primary());
                t.add_column("data", types::custom("vector(3)").nullable(false));
            });
        })
        .await?;

    let expectation = expect![[r#"
        generator client {
          provider = "prisma-client"
        }

        datasource db {
          provider = "postgresql"
          url      = "dummy-url"
        }

        model A {
          id   Int     @id @default(autoincrement())
          data Vector3
        }
    "#]];

    let extensions = ExtensionTypeConfig::new(vec![
        ExtensionType::builder()
            .prisma_name("Vector3")
            .db_name("vector")
            .db_type_modifiers(vec!["3".into()])
            .number_of_db_type_modifiers(1)
            .build(),
        ExtensionType::builder()
            .prisma_name("VectorN")
            .db_name("vector")
            .number_of_db_type_modifiers(1)
            .build(),
    ]);

    expectation.assert_eq(&api.introspect_with_extensions(&extensions).await?);

    Ok(())
}

#[test_connector(tags(Postgres), exclude(CockroachDb))]
async fn introspect_extension_type_with_modifier(api: &mut TestApi) -> TestResult {
    api.barrel()
        .execute(|migration| {
            migration.inject_custom("CREATE EXTENSION IF NOT EXISTS vector;");

            migration.create_table("A", |t| {
                t.add_column("id", types::primary());
                t.add_column("data", types::custom("vector(3)").nullable(false));
            });
        })
        .await?;

    let expectation = expect![[r#"
        generator client {
          provider = "prisma-client"
        }

        datasource db {
          provider = "postgresql"
          url      = "dummy-url"
        }

        model A {
          id   Int     @id @default(autoincrement())
          data VectorN @db.vector(3)
        }
    "#]];

    let extensions = ExtensionTypeConfig::new(vec![
        ExtensionType::builder()
            .prisma_name("VectorN")
            .db_name("vector")
            .number_of_db_type_modifiers(1)
            .build(),
    ]);

    expectation.assert_eq(&api.introspect_with_extensions(&extensions).await?);

    Ok(())
}
