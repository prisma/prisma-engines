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

#[test_connector(
    tags(Postgres),
    exclude(CockroachDb),
    preview_features("multiSchema"),
    namespaces("one", "two")
)]
fn multi_schema_drop_and_recreate_not_null_column_with_not_null_value(api: TestApi) {
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
          name String?
          @@schema("two")
        }
    "#};
    let second = base.to_owned()
        + indoc! {r#"
        model Second {
          id Int @id
          name String
          @@schema("two")
        }
    "#};

    api.schema_push(first).send().assert_green().assert_has_executed_steps();
    api.raw_cmd("INSERT INTO \"two\".\"Second\" VALUES(1, 'some value');");
    api.schema_push(second)
        .send()
        .assert_warnings(&[])
        .assert_unexecutable(&[])
        .assert_has_executed_steps();

    let mut vec_namespaces = vec![String::from("one"), String::from("two")];
    let namespaces = Namespaces::from_vec(&mut vec_namespaces);

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
fn multi_schema_drop_and_recreate_not_null_column_with_null_value(api: TestApi) {
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
          name String?
          @@schema("two")
        }
    "#};
    let second = base.to_owned()
        + indoc! {r#"
        model Second {
          id Int @id
          name String
          @@schema("two")
        }
    "#};

    api.schema_push(first).send().assert_green().assert_has_executed_steps();
    api.raw_cmd("INSERT INTO \"two\".\"Second\" VALUES(1, NULL);");
    api.schema_push(second)
        .send()
        .assert_warnings(&[])
        .assert_unexecutable(&[
            "Made the column `name` on table `Second` required, but there are 1 existing NULL values.".to_owned(),
        ])
        .assert_no_steps();

    let mut vec_namespaces = vec![String::from("one"), String::from("two")];
    let namespaces = Namespaces::from_vec(&mut vec_namespaces);

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
fn multi_schema_add_required_field_to_table(api: TestApi) {
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
    let second = base.to_owned()
        + indoc! {r#"
        model Second {
          id Int @id
          name String
          @@schema("two")
        }
    "#};

    api.schema_push(first).send().assert_green().assert_has_executed_steps();
    api.schema_push(second)
        .send()
        .assert_warnings(&[])
        .assert_unexecutable(&[])
        .assert_has_executed_steps();

    let mut vec_namespaces = vec![String::from("one"), String::from("two")];
    let namespaces = Namespaces::from_vec(&mut vec_namespaces);

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
fn multi_schema_make_field_array(api: TestApi) {
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
          name String
          @@schema("two")
        }
    "#};
    let second = base.to_owned()
        + indoc! {r#"
        model Second {
          id Int @id
          name String[]
          @@schema("two")
        }
    "#};

    api.schema_push(first).send().assert_green().assert_has_executed_steps();
    api.schema_push(second)
        .send()
        .assert_warnings(&[])
        .assert_unexecutable(&[])
        .assert_has_executed_steps();

    let mut vec_namespaces = vec![String::from("one"), String::from("two")];
    let namespaces = Namespaces::from_vec(&mut vec_namespaces);

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
fn multi_schema_remove_field_array(api: TestApi) {
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
          name String[]
          @@schema("two")
        }
    "#};
    let second = base.to_owned()
        + indoc! {r#"
        model Second {
          id Int @id
          name String
          @@schema("two")
        }
    "#};

    api.schema_push(first).send().assert_green().assert_has_executed_steps();
    api.schema_push(second)
        .send()
        .assert_warnings(&[])
        .assert_unexecutable(&[])
        .assert_has_executed_steps();

    let mut vec_namespaces = vec![String::from("one"), String::from("two")];
    let namespaces = Namespaces::from_vec(&mut vec_namespaces);

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
fn multi_schema_rename_index(api: TestApi) {
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
          name String
          @@index(fields: [name], map: "index_name")
          @@schema("two")
        }
    "#};
    let second = base.to_owned()
        + indoc! {r#"
        model Second {
          id Int @id
          name String
          @@index(fields: [name], map: "new_index_name")
          @@schema("two")
        }
    "#};

    api.schema_push(first).send().assert_green().assert_has_executed_steps();
    api.schema_push(second)
        .send()
        .assert_warnings(&[])
        .assert_unexecutable(&[])
        .assert_has_executed_steps();

    let mut vec_namespaces = vec![String::from("one"), String::from("two")];
    let namespaces = Namespaces::from_vec(&mut vec_namespaces);

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
fn multi_schema_add_unique(api: TestApi) {
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
          name String
          @@schema("two")
        }
    "#};
    let second = base.to_owned()
        + indoc! {r#"
        model Second {
          id Int @id
          name String @unique
          @@schema("two")
        }
    "#};

    api.schema_push(first).send().assert_green().assert_has_executed_steps();
    api.schema_push(second)
        .force(true)
        .send()
        .assert_warnings(&["A unique constraint covering the columns `[name]` on the table `Second` will be added. If there are existing duplicate values, this will fail.".into()])
        .assert_unexecutable(&[])
        .assert_has_executed_steps();

    let mut vec_namespaces = vec![String::from("one"), String::from("two")];
    let namespaces = Namespaces::from_vec(&mut vec_namespaces);

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
fn multi_schema_drop_enum(api: TestApi) {
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
        enum Second {
          One
          Two
          @@schema("two")
        }
    "#};
    let second = base.to_owned()
        + indoc! {r#"
        "#};

    api.schema_push(first).send().assert_green().assert_has_executed_steps();
    api.schema_push(second)
        .send()
        .assert_warnings(&[])
        .assert_unexecutable(&[])
        .assert_has_executed_steps();

    let mut vec_namespaces = vec![String::from("one"), String::from("two")];
    let namespaces = Namespaces::from_vec(&mut vec_namespaces);

    api.assert_schema_with_namespaces(namespaces)
        .assert_has_table("First")
        .assert_has_no_enum("Second");
}

#[test_connector(
    tags(Postgres),
    exclude(CockroachDb),
    preview_features("multiSchema"),
    namespaces("one", "two")
)]
fn multi_schema_drop_foreign_key(api: TestApi) {
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
    "#};
    let first = base.to_owned()
        + indoc! {r#"
        model First {
          id Int @id
          seconds Second[]
          @@schema("one")
        }
        model Second {
          id Int @id
          first_id Int
          first First? @relation(fields: [first_id], references: [id])
          @@schema("one")
        }
    "#};
    let second = base.to_owned()
        + indoc! {r#"
        model First {
          id Int @id
          @@schema("one")
        }
        model Second {
          id Int @id
          @@schema("one")
        }
        "#};

    api.schema_push(first).send().assert_green().assert_has_executed_steps();
    api.schema_push(second)
        .send()
        .assert_warnings(&[])
        .assert_unexecutable(&[])
        .assert_has_executed_steps();

    let mut vec_namespaces = vec![String::from("one"), String::from("two")];
    let namespaces = Namespaces::from_vec(&mut vec_namespaces);

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
fn multi_schema_drop_index(api: TestApi) {
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
          name String
          @@index(fields: [name], map: "index_name")
          @@schema("two")
        }
    "#};
    let second = base.to_owned()
        + indoc! {r#"
        model Second {
          id Int @id
          name String
          @@schema("two")
        }
    "#};

    api.schema_push(first).send().assert_green().assert_has_executed_steps();
    api.schema_push(second)
        .send()
        .assert_warnings(&[])
        .assert_unexecutable(&[])
        .assert_has_executed_steps();

    let mut vec_namespaces = vec![String::from("one"), String::from("two")];
    let namespaces = Namespaces::from_vec(&mut vec_namespaces);

    api.assert_schema_with_namespaces(namespaces)
        .assert_has_table("First")
        .assert_has_table("Second");
}
