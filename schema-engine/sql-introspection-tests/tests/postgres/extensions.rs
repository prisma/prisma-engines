use barrel::types;
use indoc::indoc;
use psl::parser_database::{ExtensionTypeEntry, ExtensionTypeId, ExtensionTypes};
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
          url        = "env(TEST_DATABASE_URL)"
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
          url        = "env(TEST_DATABASE_URL)"
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
          url      = "env(TEST_DATABASE_URL)"
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
          url        = "env(TEST_DATABASE_URL)"
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
          url        = "env(TEST_DATABASE_URL)"
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
          url      = "env(TEST_DATABASE_URL)"
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
          url        = "env(TEST_DATABASE_URL)"
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
          url        = "env(TEST_DATABASE_URL)"
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
          url        = "env(TEST_DATABASE_URL)"
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
          url        = "env(TEST_DATABASE_URL)"
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
          url        = "env(TEST_DATABASE_URL)"
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
          url        = "env(TEST_DATABASE_URL)"
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
          url        = "env(TEST_DATABASE_URL)"
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
          url        = "env(TEST_DATABASE_URL)"
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
          url        = "env(TEST_DATABASE_URL)"
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
          url      = "env(TEST_DATABASE_URL)"
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
          url      = "env(TEST_DATABASE_URL)"
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
          url      = "env(TEST_DATABASE_URL)"
        }

        model A {
          id   Int     @id @default(autoincrement())
          data Vector3
        }
    "#]];

    let extensions = TestExtensions {
        types: vec![("Vector3".into(), "vector".into(), 1, Some(vec!["3".into()]))],
    };

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
          url      = "env(TEST_DATABASE_URL)"
        }

        model A {
          id   Int     @id @default(autoincrement())
          data Vector3
        }
    "#]];

    let extensions = TestExtensions {
        types: vec![
            ("Vector3".into(), "vector".into(), 1, Some(vec!["3".into()])),
            ("VectorN".into(), "vector".into(), 1, None),
        ],
    };

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
          url      = "env(TEST_DATABASE_URL)"
        }

        model A {
          id   Int     @id @default(autoincrement())
          data VectorN @db.vector(3)
        }
    "#]];

    let extensions = TestExtensions {
        types: vec![("VectorN".into(), "vector".into(), 1, None)],
    };

    expectation.assert_eq(&api.introspect_with_extensions(&extensions).await?);

    Ok(())
}

struct TestExtensions {
    types: Vec<(String, String, usize, Option<Vec<String>>)>,
}

impl ExtensionTypes for TestExtensions {
    fn get_by_prisma_name(&self, name: &str) -> Option<ExtensionTypeId> {
        self.types
            .iter()
            .position(|(t, _, _, _)| t == name)
            .map(ExtensionTypeId::from)
    }

    fn get_by_db_name_and_modifiers(&self, name: &str, modifiers: Option<&[String]>) -> Option<ExtensionTypeEntry<'_>> {
        self.types
            .iter()
            .enumerate()
            .find(|(_, (_, db_name, _, db_type_modifiers))| {
                db_name == name && db_type_modifiers.as_deref() == modifiers
            })
            .or_else(|| {
                self.types
                    .iter()
                    .enumerate()
                    .find(|(_, (_, db_name, _, db_type_modifiers))| db_name == name && db_type_modifiers.is_none())
            })
            .map(
                |(i, (prisma_name, db_name, number_of_args, expected_db_type_modifiers))| ExtensionTypeEntry {
                    id: ExtensionTypeId::from(i),
                    prisma_name: prisma_name.as_str(),
                    db_namespace: None,
                    db_name: db_name.as_str(),
                    number_of_db_type_modifiers: *number_of_args,
                    db_type_modifiers: expected_db_type_modifiers.as_deref(),
                },
            )
    }

    fn enumerate(&self) -> Box<dyn Iterator<Item = psl::parser_database::ExtensionTypeEntry<'_>> + '_> {
        Box::new(self.types.iter().enumerate().map(
            |(i, (prisma_name, db_name, number_of_args, expected_db_type_modifiers))| ExtensionTypeEntry {
                id: ExtensionTypeId::from(i),
                prisma_name: prisma_name.as_str(),
                db_namespace: None,
                db_name: db_name.as_str(),
                number_of_db_type_modifiers: *number_of_args,
                db_type_modifiers: expected_db_type_modifiers.as_deref(),
            },
        ))
    }
}
