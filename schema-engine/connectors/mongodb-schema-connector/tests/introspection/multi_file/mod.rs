use crate::introspection::test_api::*;
use mongodb::bson::doc;

// Composite types
// reintrospect_removed_model_single_file
// reintrospect_removed_model_multi_file

// ----- Models -----

#[test]
fn reintrospect_new_model_single_file() {
    with_database(|mut api| async move {
        seed_model("A", &api).await?;
        seed_model("B", &api).await?;

        let input_dms = [("main.prisma", model_block_with_config("A", &api))];

        let expected = expect![[r#"
            // file: main.prisma
            generator js {
              provider        = "prisma-client-js"
              previewFeatures = []
            }

            datasource db {
              provider = "mongodb"
              url      = "env(TEST_DATABASE_URL)"
            }

            model A {
              id   String @id @default(auto()) @map("_id") @db.ObjectId
              name String
            }

            model B {
              id   String @id @default(auto()) @map("_id") @db.ObjectId
              name String
            }
        "#]];

        api.re_introspect_multi(&input_dms, expected).await;

        let expected = expect![];

        api.expect_warnings(&expected).await;

        Ok(())
    })
    .unwrap()
}

#[test]
fn reintrospect_new_model_multi_file() {
    with_database(|mut api| async move {
        seed_model("A", &api).await?;
        seed_model("B", &api).await?;
        seed_model("C", &api).await?;

        let input_dms = [
            ("a.prisma", model_block_with_config("A", &api)),
            ("b.prisma", model_block("B")),
        ];

        let expected = expect![[r#"
            // file: a.prisma
            generator js {
              provider        = "prisma-client-js"
              previewFeatures = []
            }

            datasource db {
              provider = "mongodb"
              url      = "env(TEST_DATABASE_URL)"
            }

            model A {
              id   String @id @default(auto()) @map("_id") @db.ObjectId
              name String
            }
            ------
            // file: b.prisma
            model B {
              id   String @id @default(auto()) @map("_id") @db.ObjectId
              name String
            }
            ------
            // file: introspected.prisma
            model C {
              id   String @id @default(auto()) @map("_id") @db.ObjectId
              name String
            }
        "#]];

        api.re_introspect_multi(&input_dms, expected).await;

        let expected = expect![];

        api.expect_warnings(&expected).await;

        Ok(())
    })
    .unwrap()
}

#[test]
fn reintrospect_removed_model_single_file() {
    with_database(|mut api| async move {
        seed_model("A", &api).await?;
        seed_model("B", &api).await?;

        let input_dms = [(
            "main.prisma",
            [model_block_with_config("A", &api), model_block("B"), model_block("C")].join("\n"),
        )];

        let expected = expect![[r#"
            // file: main.prisma
            generator js {
              provider        = "prisma-client-js"
              previewFeatures = []
            }

            datasource db {
              provider = "mongodb"
              url      = "env(TEST_DATABASE_URL)"
            }

            model A {
              id   String @id @default(auto()) @map("_id") @db.ObjectId
              name String
            }

            model B {
              id   String @id @default(auto()) @map("_id") @db.ObjectId
              name String
            }
        "#]];

        api.re_introspect_multi(&input_dms, expected).await;

        let expected = expect![];

        api.expect_warnings(&expected).await;

        Ok(())
    })
    .unwrap()
}

#[test]
fn reintrospect_removed_model_multi_file() {
    with_database(|mut api| async move {
        seed_model("A", &api).await?;
        seed_model("B", &api).await?;

        let input_dms = [
            ("a.prisma", model_block_with_config("A", &api)),
            ("b.prisma", model_block("B")),
            ("c.prisma", model_block("C")),
        ];

        let expected = expect![[r#"
            // file: a.prisma
            generator js {
              provider        = "prisma-client-js"
              previewFeatures = []
            }

            datasource db {
              provider = "mongodb"
              url      = "env(TEST_DATABASE_URL)"
            }

            model A {
              id   String @id @default(auto()) @map("_id") @db.ObjectId
              name String
            }
            ------
            // file: b.prisma
            model B {
              id   String @id @default(auto()) @map("_id") @db.ObjectId
              name String
            }
            ------
            // file: c.prisma

        "#]];

        api.re_introspect_multi(&input_dms, expected).await;

        let expected = expect![];

        api.expect_warnings(&expected).await;

        Ok(())
    })
    .unwrap()
}

// ----- Composite types -----

#[test]
fn reintrospect_new_composite_single_file() {
    with_database(|mut api| async move {
        seed_composite("A", &api).await?;
        seed_composite("B", &api).await?;

        let input_dms = [("main.prisma", composite_block_with_config("A", &api))];

        let expected = expect![[r#"
            // file: main.prisma
            generator js {
              provider        = "prisma-client-js"
              previewFeatures = []
            }

            datasource db {
              provider = "mongodb"
              url      = "env(TEST_DATABASE_URL)"
            }

            type AIdentity {
              firstName String
              lastName  String
            }

            type BIdentity {
              firstName String
              lastName  String
            }

            model A {
              id       String    @id @default(auto()) @map("_id") @db.ObjectId
              identity AIdentity
            }

            model B {
              id       String    @id @default(auto()) @map("_id") @db.ObjectId
              identity BIdentity
            }
        "#]];

        api.re_introspect_multi(&input_dms, expected).await;

        let expected = expect![];

        api.expect_warnings(&expected).await;

        Ok(())
    })
    .unwrap()
}

#[test]
fn reintrospect_new_composite_multi_file() {
    with_database(|mut api| async move {
        seed_composite("A", &api).await?;
        seed_composite("B", &api).await?;
        seed_composite("C", &api).await?;

        let input_dms = [
            ("a.prisma", composite_block_with_config("A", &api)),
            ("b.prisma", composite_block("B")),
        ];

        let expected = expect![[r#"
            // file: a.prisma
            generator js {
              provider        = "prisma-client-js"
              previewFeatures = []
            }

            datasource db {
              provider = "mongodb"
              url      = "env(TEST_DATABASE_URL)"
            }

            type AIdentity {
              firstName String
              lastName  String
            }

            model A {
              id       String    @id @default(auto()) @map("_id") @db.ObjectId
              identity AIdentity
            }
            ------
            // file: b.prisma
            type BIdentity {
              firstName String
              lastName  String
            }

            model B {
              id       String    @id @default(auto()) @map("_id") @db.ObjectId
              identity BIdentity
            }
            ------
            // file: introspected.prisma
            type CIdentity {
              firstName String
              lastName  String
            }

            model C {
              id       String    @id @default(auto()) @map("_id") @db.ObjectId
              identity CIdentity
            }
        "#]];

        api.re_introspect_multi(&input_dms, expected).await;

        let expected = expect![];

        api.expect_warnings(&expected).await;

        Ok(())
    })
    .unwrap()
}

#[test]
fn reintrospect_composite_model_single_file() {
    with_database(|mut api| async move {
        seed_composite("A", &api).await?;
        seed_composite("B", &api).await?;

        let input_dms = [(
            "main.prisma",
            [
                composite_block_with_config("A", &api),
                composite_block("B"),
                composite_block("C"),
            ]
            .join("\n"),
        )];

        let expected = expect![[r#"
            // file: main.prisma
            generator js {
              provider        = "prisma-client-js"
              previewFeatures = []
            }

            datasource db {
              provider = "mongodb"
              url      = "env(TEST_DATABASE_URL)"
            }

            type AIdentity {
              firstName String
              lastName  String
            }

            type BIdentity {
              firstName String
              lastName  String
            }

            model A {
              id       String    @id @default(auto()) @map("_id") @db.ObjectId
              identity AIdentity
            }

            model B {
              id       String    @id @default(auto()) @map("_id") @db.ObjectId
              identity BIdentity
            }
        "#]];

        api.re_introspect_multi(&input_dms, expected).await;

        let expected = expect![];

        api.expect_warnings(&expected).await;

        Ok(())
    })
    .unwrap()
}

#[test]
fn reintrospect_removed_composite_multi_file() {
    with_database(|mut api| async move {
        seed_composite("A", &api).await?;
        seed_composite("B", &api).await?;

        let input_dms = [
            ("a.prisma", composite_block_with_config("A", &api)),
            ("b.prisma", composite_block("B")),
            ("c.prisma", composite_block("C")),
        ];

        let expected = expect![[r#"
            // file: a.prisma
            generator js {
              provider        = "prisma-client-js"
              previewFeatures = []
            }

            datasource db {
              provider = "mongodb"
              url      = "env(TEST_DATABASE_URL)"
            }

            type AIdentity {
              firstName String
              lastName  String
            }

            model A {
              id       String    @id @default(auto()) @map("_id") @db.ObjectId
              identity AIdentity
            }
            ------
            // file: b.prisma
            type BIdentity {
              firstName String
              lastName  String
            }

            model B {
              id       String    @id @default(auto()) @map("_id") @db.ObjectId
              identity BIdentity
            }
            ------
            // file: c.prisma

        "#]];

        api.re_introspect_multi(&input_dms, expected).await;

        let expected = expect![];

        api.expect_warnings(&expected).await;

        Ok(())
    })
    .unwrap()
}

#[test]
fn reintrospect_with_existing_composite_type() {
    with_database(|mut api| async move {
        seed_composite("A", &api).await?;
        seed_composite("B", &api).await?;

        let a_dm = indoc::formatdoc! {r#"
            {config}

            model A {{
                id    String @id @default(auto()) @map("_id") @db.ObjectId
                identity Identity
            }}

            type Identity {{
                firstName String
                lastName  String
            }}
        "#,
        config = config_block_string(api.features)};

        let b_dm = indoc::formatdoc! {r#"
            model B {{
                id    String @id @default(auto()) @map("_id") @db.ObjectId
                identity Identity
            }}

            type Identity {{
                firstName String
                lastName  String
            }}
        "#};
        let input_dms = [("a.prisma", a_dm), ("b.prisma", b_dm)];

        let expected = expect![[r#"
            // file: a.prisma
            generator js {
              provider        = "prisma-client-js"
              previewFeatures = []
            }

            datasource db {
              provider = "mongodb"
              url      = "env(TEST_DATABASE_URL)"
            }

            model A {
              id       String    @id @default(auto()) @map("_id") @db.ObjectId
              identity AIdentity
            }
            ------
            // file: b.prisma
            model B {
              id       String    @id @default(auto()) @map("_id") @db.ObjectId
              identity BIdentity
            }
            ------
            // file: introspected.prisma
            type AIdentity {
              firstName String
              lastName  String
            }

            type BIdentity {
              firstName String
              lastName  String
            }
        "#]];

        api.re_introspect_multi(&input_dms, expected).await;

        let expected = expect![];

        api.expect_warnings(&expected).await;

        Ok(())
    })
    .unwrap()
}

// ----- Configuration -----

#[test]
fn reintrospect_keep_configuration_when_spread_across_files() {
    with_database(|mut api| async move {
        seed_model("A", &api).await?;
        seed_model("B", &api).await?;

        let expected = expect![[r#"
            // file: a.prisma
            datasource db {
              provider = "mongodb"
              url      = "env(TEST_DATABASE_URL)"
            }

            model A {
              id   String @id @default(auto()) @map("_id") @db.ObjectId
              name String
            }
            ------
            // file: b.prisma
            generator js {
              provider        = "prisma-client-js"
              previewFeatures = []
            }

            model B {
              id   String @id @default(auto()) @map("_id") @db.ObjectId
              name String
            }
        "#]];

        api.re_introspect_multi(
            &[
                ("a.prisma", model_block_with_datasource("A")),
                ("b.prisma", model_block_with_generator("B", &api)),
            ],
            expected,
        )
        .await;

        let expected = expect![[r#"
            // file: a.prisma
            generator js {
              provider        = "prisma-client-js"
              previewFeatures = []
            }

            model A {
              id   String @id @default(auto()) @map("_id") @db.ObjectId
              name String
            }
            ------
            // file: b.prisma
            datasource db {
              provider = "mongodb"
              url      = "env(TEST_DATABASE_URL)"
            }

            model B {
              id   String @id @default(auto()) @map("_id") @db.ObjectId
              name String
            }
        "#]];

        api.re_introspect_multi(
            &[
                ("a.prisma", model_block_with_generator("A", &api)),
                ("b.prisma", model_block_with_datasource("B")),
            ],
            expected,
        )
        .await;

        let expected = expect![];

        api.expect_warnings(&expected).await;

        Ok(())
    })
    .unwrap()
}

#[test]
fn reintrospect_keep_configuration_when_no_models() {
    with_database(|mut api| async move {
        seed_model("A", &api).await?;

        let input_dms = [
            ("a.prisma", model_block_with_datasource("A")),
            ("b.prisma", model_block_with_generator("B", &api)),
        ];

        let expected = expect![[r#"
            // file: a.prisma
            datasource db {
              provider = "mongodb"
              url      = "env(TEST_DATABASE_URL)"
            }

            model A {
              id   String @id @default(auto()) @map("_id") @db.ObjectId
              name String
            }
            ------
            // file: b.prisma
            generator js {
              provider        = "prisma-client-js"
              previewFeatures = []
            }
        "#]];

        api.re_introspect_multi(&input_dms, expected).await;

        let expected = expect![];

        api.expect_warnings(&expected).await;

        Ok(())
    })
    .unwrap()
}

#[test]
fn reintrospect_empty_multi_file() {
    with_database(|mut api| async move {
        let input_dms = [
            ("a.prisma", model_block_with_datasource("A")),
            ("b.prisma", model_block_with_generator("B", &api)),
        ];

        let expected = expect![[r#"
            // file: a.prisma
            datasource db {
              provider = "mongodb"
              url      = "env(TEST_DATABASE_URL)"
            }
            ------
            // file: b.prisma
            generator js {
              provider        = "prisma-client-js"
              previewFeatures = []
            }
        "#]];

        api.re_introspect_multi(&input_dms, expected).await;

        let expected = expect![];

        api.expect_warnings(&expected).await;

        Ok(())
    })
    .unwrap()
}

async fn seed_model(name: &str, api: &TestApi) -> Result<(), mongodb::error::Error> {
    let db = &api.db;
    db.create_collection(name, None).await?;
    let collection = db.collection(name);
    collection.insert_many(vec![doc! {"name": "John"}], None).await.unwrap();

    Ok(())
}

async fn seed_composite(name: &str, api: &TestApi) -> Result<(), mongodb::error::Error> {
    let db = &api.db;
    db.create_collection(name, None).await?;
    let collection = db.collection(name);
    collection
        .insert_many(
            vec![doc! {"identity": { "firstName": "John", "lastName": "Doe" }}],
            None,
        )
        .await
        .unwrap();

    Ok(())
}

fn model_block_with_datasource(name: &str) -> String {
    indoc::formatdoc! {r#"
        {config}

        model {name} {{
            id    String @id @default(auto()) @map("_id") @db.ObjectId
        }}
    "#,
    config = datasource_block_string()}
}

fn model_block_with_generator(name: &str, api: &TestApi) -> String {
    indoc::formatdoc! {r#"
        {config}

        model {name} {{
            id    String @id @default(auto()) @map("_id") @db.ObjectId
        }}
    "#,
    config = generator_block_string(api.features)}
}

fn model_block_with_config(name: &str, api: &TestApi) -> String {
    indoc::formatdoc! {r#"
        {config}

        model {name} {{
            id    String @id @default(auto()) @map("_id") @db.ObjectId
        }}
    "#,
    config = config_block_string(api.features)}
}

fn model_block(name: &str) -> String {
    indoc::formatdoc! {r#"
      model {name} {{
        id    String @id @default(auto()) @map("_id") @db.ObjectId
      }}
    "#}
}

fn composite_block_with_config(name: &str, api: &TestApi) -> String {
    indoc::formatdoc! {r#"
        {config}

        model {name} {{
            id    String @id @default(auto()) @map("_id") @db.ObjectId
            identity AIdentity
        }}

        type {name}Identity {{
            firstName String
            lastName  String
        }}
    "#,
    config = config_block_string(api.features)}
}

fn composite_block(name: &str) -> String {
    indoc::formatdoc! {r#"
        model {name} {{
            id    String @id @default(auto()) @map("_id") @db.ObjectId
            identity AIdentity
        }}

        type {name}Identity {{
            firstName String
            lastName  String
        }}
    "#}
}
