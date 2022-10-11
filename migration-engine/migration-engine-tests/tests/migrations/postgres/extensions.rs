use indoc::indoc;
use migration_engine_tests::test_api::*;

#[test_connector(tags(Postgres))]
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

#[test_connector(tags(Postgres))]
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

#[test_connector(tags(Postgres))]
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

#[test_connector(tags(Postgres14))]
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

#[test_connector(tags(Postgres14))]
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

#[test_connector(tags(Postgres14))]
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

#[test_connector(tags(Postgres))]
fn extension_schema_can_be_defined(api: TestApi) {
    api.raw_cmd("CREATE SCHEMA \"prisma-tests-temp\"");

    let dm = indoc! {r#"
        datasource db {
          provider   = "postgresql"
          url        = env("TEST_DATABASE_URL")
          extensions = [citext(schema: "prisma-tests-temp")]
        }

        generator js {
          provider        = "prisma-client-js"
          previewFeatures = ["postgresqlExtensions"]
        }
    "#};

    api.schema_push(dm).send().assert_green().assert_has_executed_steps();

    api.assert_schema()
        .assert_has_extension("citext")
        .assert_schema("prisma-tests-temp");
}

#[test_connector(tags(Postgres))]
fn relocatable_extension_can_be_relocated(api: TestApi) {
    api.raw_cmd("CREATE SCHEMA \"prisma-tests-temp\"");

    let dm = indoc! {r#"
        datasource db {
          provider   = "postgresql"
          url        = env("TEST_DATABASE_URL")
          extensions = [citext(schema: "prisma-tests")]
        }

        generator js {
          provider        = "prisma-client-js"
          previewFeatures = ["postgresqlExtensions"]
        }
    "#};

    api.schema_push(dm).send().assert_green().assert_has_executed_steps();

    api.assert_schema()
        .assert_has_extension("citext")
        .assert_schema("prisma-tests");

    let dm = indoc! {r#"
        datasource db {
          provider   = "postgresql"
          url        = env("TEST_DATABASE_URL")
          extensions = [citext(schema: "prisma-tests-temp")]
        }

        generator js {
          provider        = "prisma-client-js"
          previewFeatures = ["postgresqlExtensions"]
        }
    "#};

    api.schema_push(dm).send().assert_green().assert_has_executed_steps();

    api.assert_schema()
        .assert_has_extension("citext")
        .assert_schema("prisma-tests-temp");
}

#[test_connector(tags(Postgres14))]
fn non_relocatable_extension_can_be_relocated(api: TestApi) {
    api.raw_cmd("CREATE SCHEMA \"prisma-tests-temp\"");

    let dm = indoc! {r#"
        datasource db {
          provider   = "postgresql"
          url        = env("TEST_DATABASE_URL")
          extensions = [xml2(schema: "prisma-tests")]
        }

        generator js {
          provider        = "prisma-client-js"
          previewFeatures = ["postgresqlExtensions"]
        }
    "#};

    api.schema_push(dm).send().assert_green().assert_has_executed_steps();

    api.assert_schema()
        .assert_has_extension("xml2")
        .assert_schema("prisma-tests");

    let dm = indoc! {r#"
        datasource db {
          provider   = "postgresql"
          url        = env("TEST_DATABASE_URL")
          extensions = [xml2(schema: "prisma-tests-temp")]
        }

        generator js {
          provider        = "prisma-client-js"
          previewFeatures = ["postgresqlExtensions"]
        }
    "#};

    api.schema_push(dm).send().assert_green().assert_has_executed_steps();

    api.assert_schema()
        .assert_has_extension("xml2")
        .assert_schema("prisma-tests-temp");
}

#[test_connector(tags(Postgres))]
fn removing_schema_definition_does_nothing(api: TestApi) {
    let dm = indoc! {r#"
        datasource db {
          provider   = "postgresql"
          url        = env("TEST_DATABASE_URL")
          extensions = [citext(schema: "prisma-tests")]
        }

        generator js {
          provider        = "prisma-client-js"
          previewFeatures = ["postgresqlExtensions"]
        }
    "#};

    api.schema_push(dm).send().assert_green().assert_has_executed_steps();

    api.assert_schema()
        .assert_has_extension("citext")
        .assert_schema("prisma-tests");

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
        .assert_schema("prisma-tests");
}
