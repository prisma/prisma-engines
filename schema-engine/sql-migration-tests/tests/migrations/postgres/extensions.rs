use indoc::indoc;
use psl::SourceFile;
use schema_core::{ExtensionType, ExtensionTypeConfig, schema_connector::DiffTarget};
use sql_migration_tests::test_api::*;

const CONNECTOR: &dyn psl::datamodel_connector::Connector = psl::builtin_connectors::POSTGRES;

#[test_connector(tags(Postgres), exclude(CockroachDb), preview_features("postgresqlExtensions"))]
fn extensions_can_be_created(api: TestApi) {
    let dm = indoc! {r#"
        datasource db {
          provider   = "postgresql"
          url        = env("TEST_DATABASE_URL")
          extensions = [citext]
        }

        generator js {
          provider        = "prisma-client-js"
          previewFeatures = ["postgresqlExtensions"]
        }
    "#};

    api.schema_push(dm).send().assert_green().assert_has_executed_steps();

    api.assert_schema().assert_has_extension("citext");
}

#[test_connector(tags(Postgres), exclude(CockroachDb), preview_features("postgresqlExtensions"))]
fn multiple_extensions_can_be_created(api: TestApi) {
    let dm = indoc! {r#"
        datasource db {
          provider   = "postgresql"
          url        = env("TEST_DATABASE_URL")
          extensions = [citext, pg_trgm]
        }

        generator js {
          provider        = "prisma-client-js"
          previewFeatures = ["postgresqlExtensions"]
        }
    "#};

    api.schema_push(dm).send().assert_green().assert_has_executed_steps();

    api.assert_schema().assert_has_extension("citext");
    api.assert_schema().assert_has_extension("pg_trgm");
}

#[test_connector(tags(Postgres), exclude(CockroachDb), preview_features("postgresqlExtensions"))]
fn mapped_extensions_can_be_created(api: TestApi) {
    let dm = indoc! {r#"
        datasource db {
          provider   = "postgresql"
          url        = env("TEST_DATABASE_URL")
          extensions = [uuid_ossp(map: "uuid-ossp")]
        }

        generator js {
          provider        = "prisma-client-js"
          previewFeatures = ["postgresqlExtensions"]
        }
    "#};

    api.schema_push(dm).send().assert_green().assert_has_executed_steps();

    api.assert_schema().assert_has_extension("uuid-ossp");
}

#[test_connector(tags(Postgres14), exclude(CockroachDb), preview_features("postgresqlExtensions"))]
fn extensions_can_be_created_with_a_version(api: TestApi) {
    let dm = indoc! {r#"
        datasource db {
          provider   = "postgresql"
          url        = env("TEST_DATABASE_URL")
          extensions = [citext(version: "1.5")]
        }

        generator js {
          provider        = "prisma-client-js"
          previewFeatures = ["postgresqlExtensions"]
        }
    "#};

    api.schema_push(dm).send().assert_green().assert_has_executed_steps();

    api.assert_schema().assert_has_extension("citext").assert_version("1.5");
}

#[test_connector(tags(Postgres14), exclude(CockroachDb), preview_features("postgresqlExtensions"))]
fn extension_version_can_be_changed(api: TestApi) {
    let dm = indoc! {r#"
        datasource db {
          provider   = "postgresql"
          url        = env("TEST_DATABASE_URL")
          extensions = [citext(version: "1.5")]
        }

        generator js {
          provider        = "prisma-client-js"
          previewFeatures = ["postgresqlExtensions"]
        }
    "#};

    api.schema_push(dm).send().assert_green().assert_has_executed_steps();

    api.assert_schema().assert_has_extension("citext").assert_version("1.5");

    let dm = indoc! {r#"
        datasource db {
          provider   = "postgresql"
          url        = env("TEST_DATABASE_URL")
          extensions = [citext(version: "1.6")]
        }

        generator js {
          provider        = "prisma-client-js"
          previewFeatures = ["postgresqlExtensions"]
        }
    "#};

    api.schema_push(dm).send().assert_green().assert_has_executed_steps();

    api.assert_schema().assert_has_extension("citext").assert_version("1.6");
}

#[test_connector(tags(Postgres14), exclude(CockroachDb), preview_features("postgresqlExtensions"))]
fn extension_version_does_not_change_on_empty(api: TestApi) {
    let dm = indoc! {r#"
        datasource db {
          provider   = "postgresql"
          url        = env("TEST_DATABASE_URL")
          extensions = [citext(version: "1.5")]
        }

        generator js {
          provider        = "prisma-client-js"
          previewFeatures = ["postgresqlExtensions"]
        }
    "#};

    api.schema_push(dm).send().assert_green().assert_has_executed_steps();

    api.assert_schema().assert_has_extension("citext").assert_version("1.5");

    let dm = indoc! {r#"
        datasource db {
          provider   = "postgresql"
          url        = env("TEST_DATABASE_URL")
          extensions = [citext]
        }

        generator js {
          provider        = "prisma-client-js"
          previewFeatures = ["postgresqlExtensions"]
        }
    "#};

    api.schema_push(dm).send().assert_green().assert_no_steps();

    api.assert_schema().assert_has_extension("citext").assert_version("1.5");
}

#[test_connector(tags(Postgres), exclude(CockroachDb), preview_features("postgresqlExtensions"))]
fn extension_schema_can_be_defined(api: TestApi) {
    api.raw_cmd("CREATE SCHEMA \"public-temp\"");

    let dm = indoc! {r#"
        datasource db {
          provider   = "postgresql"
          url        = env("TEST_DATABASE_URL")
          extensions = [citext(schema: "public-temp")]
        }

        generator js {
          provider        = "prisma-client-js"
          previewFeatures = ["postgresqlExtensions"]
        }
    "#};

    api.schema_push(dm).send().assert_green().assert_has_executed_steps();

    api.assert_schema()
        .assert_has_extension("citext")
        .assert_schema("public-temp");
}

#[test_connector(tags(Postgres), exclude(CockroachDb), preview_features("postgresqlExtensions"))]
fn relocatable_extension_can_be_relocated(api: TestApi) {
    api.raw_cmd("CREATE SCHEMA \"public-temp\"");

    let dm = indoc! {r#"
        datasource db {
          provider   = "postgresql"
          url        = env("TEST_DATABASE_URL")
          extensions = [citext(schema: "public")]
        }

        generator js {
          provider        = "prisma-client-js"
          previewFeatures = ["postgresqlExtensions"]
        }
    "#};

    api.schema_push(dm).send().assert_green().assert_has_executed_steps();

    api.assert_schema()
        .assert_has_extension("citext")
        .assert_schema("public");

    let dm = indoc! {r#"
        datasource db {
          provider   = "postgresql"
          url        = env("TEST_DATABASE_URL")
          extensions = [citext(schema: "public-temp")]
        }

        generator js {
          provider        = "prisma-client-js"
          previewFeatures = ["postgresqlExtensions"]
        }
    "#};

    api.schema_push(dm).send().assert_green().assert_has_executed_steps();

    api.assert_schema()
        .assert_has_extension("citext")
        .assert_schema("public-temp");
}

#[test_connector(tags(Postgres14), exclude(CockroachDb), preview_features("postgresqlExtensions"))]
fn non_relocatable_extension_can_be_relocated(api: TestApi) {
    api.raw_cmd("CREATE SCHEMA \"public-temp\"");

    let dm = indoc! {r#"
        datasource db {
          provider   = "postgresql"
          url        = env("TEST_DATABASE_URL")
          extensions = [xml2(schema: "public")]
        }

        generator js {
          provider        = "prisma-client-js"
          previewFeatures = ["postgresqlExtensions"]
        }
    "#};

    api.schema_push(dm).send().assert_green().assert_has_executed_steps();

    api.assert_schema().assert_has_extension("xml2").assert_schema("public");

    let dm = indoc! {r#"
        datasource db {
          provider   = "postgresql"
          url        = env("TEST_DATABASE_URL")
          extensions = [xml2(schema: "public-temp")]
        }

        generator js {
          provider        = "prisma-client-js"
          previewFeatures = ["postgresqlExtensions"]
        }
    "#};

    api.schema_push(dm).send().assert_green().assert_has_executed_steps();

    api.assert_schema()
        .assert_has_extension("xml2")
        .assert_schema("public-temp");
}

#[test_connector(tags(Postgres), exclude(CockroachDb), preview_features("postgresqlExtensions"))]
fn removing_schema_definition_does_nothing(api: TestApi) {
    let dm = indoc! {r#"
        datasource db {
          provider   = "postgresql"
          url        = env("TEST_DATABASE_URL")
          extensions = [citext(schema: "public")]
        }

        generator js {
          provider        = "prisma-client-js"
          previewFeatures = ["postgresqlExtensions"]
        }
    "#};

    api.schema_push(dm).send().assert_green().assert_has_executed_steps();

    api.assert_schema()
        .assert_has_extension("citext")
        .assert_schema("public");

    let dm = indoc! {r#"
        datasource db {
          provider   = "postgresql"
          url        = env("TEST_DATABASE_URL")
          extensions = [citext]
        }

        generator js {
          provider        = "prisma-client-js"
          previewFeatures = ["postgresqlExtensions"]
        }
    "#};

    api.schema_push(dm).send().assert_green().assert_no_steps();

    api.assert_schema()
        .assert_has_extension("citext")
        .assert_schema("public");
}

#[test_connector(tags(Postgres), exclude(CockroachDb), preview_features("postgresqlExtensions"))]
fn extension_functions_can_be_used_in_the_same_migration(api: TestApi) {
    let dm = indoc! {r#"
        datasource db {
          provider   = "postgresql"
          url        = env("TEST_DATABASE_URL")
          extensions = [uuid_ossp(map: "uuid-ossp")]
        }

        generator js {
          provider        = "prisma-client-js"
          previewFeatures = ["postgresqlExtensions"]
        }

        model A {
          id String @id @db.Uuid @default(dbgenerated("uuid_generate_v4()"))
        }
    "#};

    api.schema_push(dm).send().assert_green().assert_has_executed_steps();

    api.assert_schema().assert_has_extension("uuid-ossp");

    api.assert_schema().assert_table("A", |table| {
        table.assert_column("id", |col| col.assert_dbgenerated("uuid_generate_v4()"))
    });
}

#[test_connector(tags(Postgres), exclude(CockroachDb))]
fn create_table_with_extension_types(api: TestApi) {
    let dm = indoc![
        r#"
        generator client {
          provider        = "prisma-client-js"
        }

        datasource db {
          provider = "postgresql"
          url      = "env(TEST_DATABASE_URL)"
        }

        model A {
          id   Int     @id @default(autoincrement())
          data Vector3
        }
    "#
    ];

    let extensions = ExtensionTypeConfig::new(vec![
        ExtensionType::builder()
            .prisma_name("Vector3")
            .db_name("vector")
            .db_type_modifiers(vec!["3".into()])
            .number_of_db_type_modifiers(1)
            .build(),
    ]);

    api.raw_cmd("CREATE EXTENSION IF NOT EXISTS vector");

    api.schema_push(dm)
        .extensions(&extensions)
        .send()
        .assert_green()
        .assert_has_executed_steps();

    api.assert_schema().assert_table("A", |table| {
        table.assert_column("data", |col| {
            col.assert_full_data_type("vector")
                .assert_native_type("vector(3)", CONNECTOR)
        })
    });
}

#[test_connector(tags(Postgres), exclude(CockroachDb))]
fn diff_extension_type_changed_modifiers(api: TestApi) {
    let dm1 = indoc! {r#"
        generator client {
          provider = "prisma-client-js"
        }

        datasource db {
          provider = "postgresql"
          url      = "env(TEST_DATABASE_URL)"
        }

        model A {
          id   Int     @id @default(autoincrement())
          data Vector3
        }
    "#};

    let dm2 = indoc! {r#"
        generator client {
          provider = "prisma-client-js"
        }

        datasource db {
          provider = "postgresql"
          url      = "env(TEST_DATABASE_URL)"
        }

        model A {
          id   Int     @id @default(autoincrement())
          data Vector4
        }
    "#};

    let extensions = ExtensionTypeConfig::new(vec![
        ExtensionType::builder()
            .prisma_name("Vector3")
            .db_name("vector")
            .db_type_modifiers(vec!["3".into()])
            .number_of_db_type_modifiers(1)
            .build(),
        ExtensionType::builder()
            .prisma_name("Vector4")
            .db_name("vector")
            .db_type_modifiers(vec!["4".into()])
            .number_of_db_type_modifiers(1)
            .build(),
    ]);

    api.raw_cmd("CREATE EXTENSION IF NOT EXISTS vector");

    let diff = api.connector_diff(
        DiffTarget::Datamodel(vec![("schema.prisma".into(), SourceFile::new_static(dm1))], &extensions),
        DiffTarget::Datamodel(vec![("schema.prisma".into(), SourceFile::new_static(dm2))], &extensions),
        None,
    );

    expect![[r#"
        -- AlterTable
        ALTER TABLE "public"."A" ALTER COLUMN "data" SET DATA TYPE vector(4);
    "#]]
    .assert_eq(&diff);
}

#[test_connector(tags(Postgres), exclude(CockroachDb))]
fn diff_extension_type_unchanged_modifiers(api: TestApi) {
    let dm = indoc! {r#"
        generator client {
          provider = "prisma-client-js"
        }

        datasource db {
          provider = "postgresql"
          url = "env(TEST_DATABASE_URL)"
        }

        model A {
          id   Int     @id @default(autoincrement())
          data Vector3
        }
    "#};

    let extensions = ExtensionTypeConfig::new(vec![
        ExtensionType::builder()
            .prisma_name("Vector3")
            .db_name("vector")
            .db_type_modifiers(vec!["3".into()])
            .number_of_db_type_modifiers(1)
            .build(),
    ]);

    api.raw_cmd("CREATE EXTENSION IF NOT EXISTS vector");

    let diff = api.connector_diff(
        DiffTarget::Datamodel(vec![("schema.prisma".into(), SourceFile::new_static(dm))], &extensions),
        DiffTarget::Datamodel(vec![("schema.prisma".into(), SourceFile::new_static(dm))], &extensions),
        None,
    );

    expect![[r#"
    -- This is an empty migration."#]]
    .assert_eq(&diff);
}

#[test_connector(tags(Postgres), exclude(CockroachDb))]
fn diff_extension_type_changed_db_type_modifiers(api: TestApi) {
    let dm1 = indoc! {r#"
        generator client {
          provider = "prisma-client-js"
        }

        datasource db {
          provider = "postgresql"
          url      = "env(TEST_DATABASE_URL)"
        }

        model A {
          id   Int     @id @default(autoincrement())
          data VectorN @db.vector(3)
        }
    "#};

    let dm2 = indoc! {r#"
        generator client {
          provider = "prisma-client-js"
        }

        datasource db {
          provider = "postgresql"
          url      = "env(TEST_DATABASE_URL)"
        }

        model A {
          id   Int     @id @default(autoincrement())
          data VectorN @db.vector(4)
        }
    "#};

    let extensions = ExtensionTypeConfig::new(vec![
        ExtensionType::builder()
            .prisma_name("VectorN")
            .db_name("vector")
            .number_of_db_type_modifiers(1)
            .build(),
    ]);

    api.raw_cmd("CREATE EXTENSION IF NOT EXISTS vector");

    let diff = api.connector_diff(
        DiffTarget::Datamodel(vec![("schema.prisma".into(), SourceFile::new_static(dm1))], &extensions),
        DiffTarget::Datamodel(vec![("schema.prisma".into(), SourceFile::new_static(dm2))], &extensions),
        None,
    );

    expect![[r#"
        -- AlterTable
        ALTER TABLE "public"."A" ALTER COLUMN "data" SET DATA TYPE vector(4);
    "#]]
    .assert_eq(&diff);
}

#[test_connector(tags(Postgres), exclude(CockroachDb))]
fn diff_extension_type_unchanged_db_type_modifiers(api: TestApi) {
    let dm = indoc! {r#"
        generator client {
          provider = "prisma-client-js"
        }

        datasource db {
          provider = "postgresql"
          url      = "env(TEST_DATABASE_URL)"
        }

        model A {
          id   Int     @id @default(autoincrement())
          data VectorN @db.vector(3)
        }
    "#};

    let extensions = ExtensionTypeConfig::new(vec![
        ExtensionType::builder()
            .prisma_name("VectorN")
            .db_name("vector")
            .number_of_db_type_modifiers(1)
            .build(),
    ]);

    api.raw_cmd("CREATE EXTENSION IF NOT EXISTS vector");

    let diff = api.connector_diff(
        DiffTarget::Datamodel(vec![("schema.prisma".into(), SourceFile::new_static(dm))], &extensions),
        DiffTarget::Datamodel(vec![("schema.prisma".into(), SourceFile::new_static(dm))], &extensions),
        None,
    );

    expect![[r#"
    -- This is an empty migration."#]]
    .assert_eq(&diff);
}
