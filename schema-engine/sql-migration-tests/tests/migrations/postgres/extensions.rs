use indoc::indoc;
use sql_migration_tests::test_api::*;

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
