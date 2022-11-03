use indoc::indoc;
use migration_core::migration_connector::Namespaces;
use migration_engine_tests::test_api::*;

#[test_connector(
    tags(Postgres),
    exclude(CockroachDb),
    preview_features("multiSchema"),
    namespaces("one", "two")
)]
fn multi_schema_basic(api: TestApi) {
    let dm = indoc! {r#"
        datasource db {
          provider   = "postgresql"
          url        = env("TEST_DATABASE_URL")
          schemas    = ["one", "two"]
        }

        generator js {
          provider        = "prisma-client-js"
          previewFeatures = ["multiSchema"]
        }

        model First {
          id Int @id
          @@schema("one")
        }

        model Second {
          id Int @id
          @@schema("two")
        }
    "#};

    let mut vec_namespaces = vec![String::from("one"), String::from("two")];
    let namespaces = Namespaces::from_vec(&mut vec_namespaces);

    api.schema_push(dm).send().assert_green().assert_has_executed_steps();

    api.assert_schema_with_namespaces(namespaces)
        .assert_has_table("First")
        .assert_has_table("Second");
}

#[test_connector(
    tags(Postgres),
    exclude(CockroachDb),
    preview_features("multiSchema"),
    namespaces("one", "two")
)]
fn multi_schema_idempotent(api: TestApi) {
    let dm = indoc! {r#"
        datasource db {
          provider   = "postgresql"
          url        = env("TEST_DATABASE_URL")
          schemas    = ["one", "two"]
        }

        generator js {
          provider        = "prisma-client-js"
          previewFeatures = ["multiSchema"]
        }

        model First {
          id Int @id
          @@schema("one")
        }

        model Second {
          id Int @id
          @@schema("two")
        }
    "#};

    api.schema_push(dm).send().assert_green().assert_has_executed_steps();
    api.schema_push(dm).send().assert_green().assert_no_steps();
}

#[test_connector(
    tags(Postgres),
    exclude(CockroachDb),
    preview_features("multiSchema"),
    namespaces("one", "two")
)]
fn multi_schema_add_table(api: TestApi) {
    let first = indoc! {r#"
        datasource db {
          provider   = "postgresql"
          url        = env("TEST_DATABASE_URL")
          schemas    = ["one", "two"]
        }

        generator js {
          provider        = "prisma-client-js"
          previewFeatures = ["multiSchema"]
        }

        model First {
          id Int @id
          @@schema("one")
        }

        model Second {
          id Int @id
          @@schema("two")
        }
    "#};
    let second = first.to_owned()
        + indoc! {r#"

        model Third {
          id Int @id
          @@schema("one")
        }
    "#};

    api.schema_push(first).send().assert_green().assert_has_executed_steps();
    api.schema_push(second)
        .send()
        .assert_green()
        .assert_has_executed_steps();

    let mut vec_namespaces = vec![String::from("one"), String::from("two")];
    let namespaces = Namespaces::from_vec(&mut vec_namespaces);

    api.assert_schema_with_namespaces(namespaces)
        .assert_has_table("First")
        .assert_has_table("Second")
        .assert_has_table("Third");
}

#[test_connector(
    tags(Postgres),
    exclude(CockroachDb),
    preview_features("multiSchema"),
    namespaces("one", "two")
)]
fn multi_schema_remove_table(api: TestApi) {
    let base = indoc! {r#"
        datasource db {
          provider   = "postgresql"
          url        = env("TEST_DATABASE_URL")
          schemas    = ["one", "two"]
        }
        generator js {
          provider        = "prisma-client-js"
          previewFeatures = ["multiSchema"]
        }
        model First {
          id Int @id
          @@schema("one")
        }
    "#};
    let first = base.to_owned()
        + indoc! {r#"
        model Second {
          id Int @id
          @@schema("two")
        }
    "#};
    let second = base;

    api.schema_push(first).send().assert_green().assert_has_executed_steps();
    api.schema_push(second)
        .send()
        .assert_warnings(&[])
        .assert_has_executed_steps();

    let mut vec_namespaces = vec![String::from("one"), String::from("two")];
    let namespaces = Namespaces::from_vec(&mut vec_namespaces);

    api.assert_schema_with_namespaces(namespaces)
        .assert_has_table("First")
        .assert_has_no_table("Second");
}
