use sql_introspection_tests::{test_api::*, TestResult};

#[test_connector(tags(Mysql))]
async fn multiple_schemas_without_schema_property_are_not_introspected(api: &mut TestApi) -> TestResult {
    let schema_name = api.schema_name();
    let other_name = "not_inspected_schema";
    let create_table = format!("CREATE TABLE `{schema_name}`.`A` (id Int PRIMARY KEY, data Text)",);
    let create_primary = format!("CREATE INDEX `A_idx` ON `{schema_name}`.`A` (`data`(128))",);

    api.database().raw_cmd(&create_table).await?;
    api.database().raw_cmd(&create_primary).await?;

    // Need to manually drop the schema to cleanup from prior tests as this test cannot rely on the usual
    // cleanup logic through test_connector(..namespaces(...)))
    let drop_schema = format!("DROP Schema IF EXISTS `{other_name}`",);
    let create_schema = format!("CREATE Schema `{other_name}`",);
    let create_table = format!("CREATE TABLE `{other_name}`.`B` (id Int PRIMARY KEY, data Text)",);
    let create_primary = format!("CREATE INDEX `B_idx` ON `{other_name}`.`B` (`data`(128))",);

    api.database().raw_cmd(&drop_schema).await?;
    api.database().raw_cmd(&create_schema).await?;
    api.database().raw_cmd(&create_table).await?;
    api.database().raw_cmd(&create_primary).await?;

    let expected = expect![[r#"
        model A {
          id   Int     @id
          data String? @db.Text

          @@index([data(length: 128)], map: "A_idx")
        }
    "#]];

    let result = api.introspect_dml().await?;
    expected.assert_eq(&result);

    Ok(())
}

#[test_connector(tags(Mysql), preview_features("multiSchema"), namespaces("first", "second"))]
async fn multiple_schemas_w_tables_are_introspected(api: &mut TestApi) -> TestResult {
    let schema_name = &api.namespaces()[0];
    let other_name = &api.namespaces()[1];
    let create_schema = format!("CREATE Schema `{schema_name}`",);
    let create_table = format!("CREATE TABLE `{schema_name}`.`A` (id Int PRIMARY KEY, data Text)",);
    let create_primary = format!("CREATE INDEX `A_idx` ON `{schema_name}`.`A` (`data`(128))",);

    api.database().raw_cmd(&create_schema).await?;
    api.database().raw_cmd(&create_table).await?;
    api.database().raw_cmd(&create_primary).await?;

    let create_schema = format!("CREATE Schema `{other_name}`",);
    let create_table = format!("CREATE TABLE `{other_name}`.`B` (id Int PRIMARY KEY, data Text)",);
    let create_primary = format!("CREATE INDEX `B_idx` ON `{other_name}`.`B` (`data`(128))",);

    api.database().raw_cmd(&create_schema).await?;
    api.database().raw_cmd(&create_table).await?;
    api.database().raw_cmd(&create_primary).await?;

    let expected = formatdoc! {r#"
        model A {{
          id   Int     @id
          data String? @db.Text

          @@index([data(length: 128)], map: "A_idx")
          @@schema("{schema_name}")
        }}

        model B {{
          id   Int     @id
          data String? @db.Text

          @@index([data(length: 128)], map: "B_idx")
          @@schema("{other_name}")
        }}
    "#};

    let result = api.introspect_dml().await?;
    pretty_assertions::assert_eq!(expected, result);

    Ok(())
}

#[test_connector(tags(Mysql), preview_features("multiSchema"), namespaces("first", "second"))]
async fn multiple_schemas_w_tables_are_reintrospected(api: &mut TestApi) -> TestResult {
    let schema_name = &api.namespaces()[0];
    let other_name = &api.namespaces()[1];
    let create_schema = format!("CREATE Schema `{schema_name}`",);
    let create_table = format!("CREATE TABLE `{schema_name}`.`A` (id Int PRIMARY KEY, data Text)",);
    let create_primary = format!("CREATE INDEX `A_idx` ON `{schema_name}`.`A` (`data`(128))",);

    api.database().raw_cmd(&create_schema).await?;
    api.database().raw_cmd(&create_table).await?;
    api.database().raw_cmd(&create_primary).await?;

    let create_schema = format!("CREATE Schema `{other_name}`",);
    let create_table = format!("CREATE TABLE `{other_name}`.`B` (id Int PRIMARY KEY, data Text)",);
    let create_primary = format!("CREATE INDEX `B_idx` ON `{other_name}`.`B` (`data`(128))",);

    api.database().raw_cmd(&create_schema).await?;
    api.database().raw_cmd(&create_table).await?;
    api.database().raw_cmd(&create_primary).await?;

    let input = format!(
        r#"
        model A {{
          id   Int     @id
          data String? @db.Text

          @@index([data(length: 128)], map: "A_idx")
          @@schema("{schema_name}")
        }}

        model B {{
          id   Int     @id
          data String? @db.Text

          @@index([data(length: 128)], map: "B_idx")
          @@schema("{other_name}")
        }}
    "#
    );

    let expected = expect![[r#"
        model A {
          id   Int     @id
          data String? @db.Text

          @@index([data(length: 128)], map: "A_idx")
          @@schema("first")
        }

        model B {
          id   Int     @id
          data String? @db.Text

          @@index([data(length: 128)], map: "B_idx")
          @@schema("second")
        }
    "#]];

    api.expect_re_introspected_datamodel(&input, expected).await;

    Ok(())
}

#[test_connector(tags(Mysql), preview_features("multiSchema"), namespaces("first", "second"))]
async fn multiple_schemas_w_duplicate_table_names_are_introspected(api: &mut TestApi) -> TestResult {
    let schema_name = "first";
    let other_name = "second";
    let setup = formatdoc! {
        r#"
             CREATE SCHEMA `{schema_name}`;
             CREATE TABLE `{schema_name}`.`A` (id INT PRIMARY KEY);

             CREATE SCHEMA `{other_name}`;
             CREATE TABLE `{other_name}`.`A` (id INT PRIMARY KEY);
         "#
    };
    api.raw_cmd(&setup).await;

    let expected = expect![[r#"
        generator client {
          provider        = "prisma-client-js"
          previewFeatures = ["multiSchema"]
        }

        datasource db {
          provider = "mysql"
          url      = "env(TEST_DATABASE_URL)"
          schemas  = ["first", "second"]
        }

        model first_A {
          id Int @id

          @@map("A")
          @@schema("first")
        }

        model second_A {
          id Int @id

          @@map("A")
          @@schema("second")
        }
    "#]];

    api.expect_datamodel(&expected).await;

    let expected = expect![[r#"
        *** WARNING ***

        These items were renamed due to their names being duplicates in the Prisma schema:
          - Type: "model", name: "first_A"
          - Type: "model", name: "second_A"
    "#]];

    api.expect_warnings(&expected).await;

    Ok(())
}

#[test_connector(tags(Mysql), preview_features("multiSchema"), namespaces("1first", "2second"))]
async fn multiple_schemas_w_duplicate_sanitized_table_names_are_introspected(api: &mut TestApi) -> TestResult {
    let schema_name = "1first";
    let other_name = "2second";
    let setup = formatdoc! {
        r#"
             CREATE SCHEMA `{schema_name}`;
             CREATE TABLE `{schema_name}`.`2A` (id INT PRIMARY KEY);

             CREATE SCHEMA `{other_name}`;
             CREATE TABLE `{other_name}`.`1A` (id INT PRIMARY KEY);
         "#
    };
    api.raw_cmd(&setup).await;

    let expected = expect![[r#"
        generator client {
          provider        = "prisma-client-js"
          previewFeatures = ["multiSchema"]
        }

        datasource db {
          provider = "mysql"
          url      = "env(TEST_DATABASE_URL)"
          schemas  = ["1first", "2second"]
        }

        model first_2A {
          id Int @id

          @@map("2A")
          @@schema("1first")
        }

        model second_1A {
          id Int @id

          @@map("1A")
          @@schema("2second")
        }
    "#]];

    api.expect_datamodel(&expected).await;

    let expected = expect![[r#"
        *** WARNING ***

        These items were renamed due to their names being duplicates in the Prisma schema:
          - Type: "model", name: "first_2A"
          - Type: "model", name: "second_1A"
    "#]];

    api.expect_warnings(&expected).await;

    Ok(())
}

#[test_connector(tags(Mysql), preview_features("multiSchema"), namespaces("first", "second"))]
async fn multiple_schemas_w_cross_schema_are_introspected(api: &mut TestApi) -> TestResult {
    let schema_name = "first";
    let other_name = "second";
    let create_schema = format!("CREATE Schema `{schema_name}`",);
    let create_table = format!("CREATE TABLE `{schema_name}`.`A` (id INT PRIMARY KEY)",);
    //Todo
    api.database().raw_cmd(&create_schema).await?;
    api.database().raw_cmd(&create_table).await?;

    let create_schema = format!("CREATE Schema `{other_name}`",);
    let create_table =
        format!("CREATE TABLE `{other_name}`.`B` (id INT PRIMARY KEY, fk INT, FOREIGN KEY (fk) REFERENCES `{schema_name}`.`A`(`id`))",);

    api.database().raw_cmd(&create_schema).await?;
    api.database().raw_cmd(&create_table).await?;

    let expected = expect![[r#"
        model A {
          id Int @id
          B  B[]

          @@schema("first")
        }

        model B {
          id Int  @id
          fk Int?
          A  A?   @relation(fields: [fk], references: [id], onDelete: NoAction, onUpdate: NoAction, map: "B_ibfk_1")

          @@index([fk], map: "fk")
          @@schema("second")
        }
    "#]];

    let result = api.introspect_dml().await?;
    expected.assert_eq(&result);

    Ok(())
}

#[test_connector(tags(Mysql), preview_features("multiSchema"), namespaces("first", "second"))]
async fn multiple_schemas_w_cross_schema_are_reintrospected(api: &mut TestApi) -> TestResult {
    let schema_name = "first";
    let other_name = "second";
    let create_schema = format!("CREATE Schema `{schema_name}`",);
    let create_table = format!("CREATE TABLE `{schema_name}`.`A` (id Int PRIMARY KEY)",);
    //Todo
    api.database().raw_cmd(&create_schema).await?;
    api.database().raw_cmd(&create_table).await?;

    let create_schema = format!("CREATE Schema `{other_name}`",);
    let create_table =
        format!("CREATE TABLE `{other_name}`.`B` (id Int PRIMARY KEY, fk Int, FOREIGN KEY (fk) REFERENCES `{schema_name}`.`A`(`id`))",);

    api.database().raw_cmd(&create_schema).await?;
    api.database().raw_cmd(&create_table).await?;

    let input = indoc! {r#"
        model A {
          id Int @id
          B  B[]

          @@schema("first")
        }

        model B {
          id Int  @id
          fk Int?
          A  A?   @relation(fields: [fk], references: [id], onDelete: NoAction, onUpdate: NoAction)

          @@schema("first")
        }
    "#};

    let expected = expect![[r#"
        model A {
          id Int @id
          B  B[]

          @@schema("first")
        }

        model B {
          id Int  @id
          fk Int?
          A  A?   @relation(fields: [fk], references: [id], onDelete: NoAction, onUpdate: NoAction, map: "B_ibfk_1")

          @@index([fk], map: "fk")
          @@schema("second")
        }
    "#]];

    api.expect_re_introspected_datamodel(input, expected).await;

    Ok(())
}

#[test_connector(tags(Mysql), preview_features("multiSchema"), namespaces("first", "second"))]
async fn multiple_schemas_w_cross_schema_fks_w_duplicate_names_are_introspected(api: &mut TestApi) -> TestResult {
    let schema_name = "first";
    let other_name = "second";
    let create_schema = format!("CREATE SCHEMA `{schema_name}`",);
    let create_table = format!("CREATE TABLE `{schema_name}`.`A` (id INT PRIMARY KEY)",);
    //Todo
    api.database().raw_cmd(&create_schema).await?;
    api.database().raw_cmd(&create_table).await?;

    let create_schema = format!("CREATE SCHEMA `{other_name}`",);
    let create_table =
        format!("CREATE TABLE `{other_name}`.`A` (id INT PRIMARY KEY, fk INT, FOREIGN KEY (fk) REFERENCES `{schema_name}`.`A`(`id`))",);

    api.database().raw_cmd(&create_schema).await?;
    api.database().raw_cmd(&create_table).await?;

    let expected = expect![[r#"
        model first_A {
          id Int        @id
          A  second_A[]

          @@map("A")
          @@schema("first")
        }

        model second_A {
          id Int      @id
          fk Int?
          A  first_A? @relation(fields: [fk], references: [id], onDelete: NoAction, onUpdate: NoAction, map: "A_ibfk_1")

          @@index([fk], map: "fk")
          @@map("A")
          @@schema("second")
        }
    "#]];

    let result = api.introspect_dml().await?;
    expected.assert_eq(&result);

    Ok(())
}

// #[test_connector(tags(Mysql), preview_features("multiSchema"), namespaces("first", "second_schema"))]
// async fn multiple_schemas_w_enums_are_introspected(api: &mut TestApi) -> TestResult {
//     let schema_name = "first";
//     let other_name = "second_schema";
//     let sql = format! {
//         r#"
//             CREATE SCHEMA "{schema_name}";
//             CREATE TYPE "{schema_name}"."HappyMood" AS ENUM ('happy');
//             CREATE SCHEMA "{other_name}";
//             CREATE TYPE "{other_name}"."SadMood" AS ENUM ('sad');
//         "#,
//     };

//     api.raw_cmd(&sql).await;

//     let expected = expect![[r#"
//         generator client {
//           provider        = "prisma-client-js"
//           previewFeatures = ["multiSchema"]
//         }

//         datasource db {
//           provider = "mysql"
//           url      = "env(TEST_DATABASE_URL)"
//           schemas  = ["first", "second_schema"]
//         }

//         enum HappyMood {
//           happy

//           @@schema("first")
//         }

//         enum SadMood {
//           sad

//           @@schema("second_schema")
//         }
//     "#]];

//     api.expect_datamodel(&expected).await;
//     Ok(())
// }

// #[test_connector(tags(Mysql), preview_features("multiSchema"), namespaces("first", "second"))]
// async fn multiple_schemas_w_duplicate_enums_are_introspected(api: &mut TestApi) -> TestResult {
//     let schema_name = "first";
//     let other_name = "second";
//     let setup = formatdoc! {
//         r#"
//             CREATE SCHEMA "{schema_name}";
//             CREATE TYPE "{schema_name}"."HappyMood" AS ENUM ('happy');
//             CREATE TABLE "{schema_name}"."HappyPerson" (mood "{schema_name}"."HappyMood" PRIMARY KEY);

//             CREATE SCHEMA "{other_name}";
//             CREATE TYPE "{other_name}"."HappyMood" AS ENUM ('veryHappy');
//             CREATE TABLE "{other_name}"."VeryHappyPerson" (mood "{other_name}"."HappyMood" PRIMARY KEY);
//             CREATE TABLE "{other_name}"."HappyPerson" (mood "{schema_name}"."HappyMood" PRIMARY KEY);

//         "#
//     };

//     api.raw_cmd(&setup).await;

//     let expected = expect![[r#"
//         generator client {
//           provider        = "prisma-client-js"
//           previewFeatures = ["multiSchema"]
//         }

//         datasource db {
//           provider = "mysql"
//           url      = "env(TEST_DATABASE_URL)"
//           schemas  = ["first", "second"]
//         }

//         model first_HappyPerson {
//           mood first_HappyMood @id

//           @@map("HappyPerson")
//           @@schema("first")
//         }

//         model second_HappyPerson {
//           mood first_HappyMood @id

//           @@map("HappyPerson")
//           @@schema("second")
//         }

//         model VeryHappyPerson {
//           mood second_HappyMood @id

//           @@schema("second")
//         }

//         enum first_HappyMood {
//           happy

//           @@map("HappyMood")
//           @@schema("first")
//         }

//         enum second_HappyMood {
//           veryHappy

//           @@map("HappyMood")
//           @@schema("second")
//         }
//     "#]];

//     api.expect_datamodel(&expected).await;

//     let expected = expect![[r#"
//         *** WARNING ***

//         These items were renamed due to their names being duplicates in the Prisma schema:
//           - Type: "enum", name: "first_HappyMood"
//           - Type: "enum", name: "second_HappyMood"
//           - Type: "model", name: "first_HappyPerson"
//           - Type: "model", name: "second_HappyPerson"
//     "#]];

//     api.expect_warnings(&expected).await;

//     Ok(())
// }

// #[test_connector(tags(Mysql), preview_features("multiSchema"), namespaces("first", "second"))]
// async fn multiple_schemas_w_duplicate_models_are_reintrospected(api: &mut TestApi) -> TestResult {
//     let schema_name = "first";
//     let other_name = "second";
//     let setup = formatdoc! {
//         r#"
//             CREATE SCHEMA "{schema_name}";
//             CREATE TABLE "{schema_name}"."HappyPerson" (id SERIAL PRIMARY KEY);

//             CREATE SCHEMA "{other_name}";
//             CREATE TABLE "{other_name}"."HappyPerson" (id SERIAL PRIMARY KEY);

//         "#
//     };

//     api.raw_cmd(&setup).await;

//     let input = indoc! {r#"
//         model FooBar {
//           id Int @id @default(autoincrement())

//           @@map("HappyPerson")
//           @@schema("first")
//         }

//         model HappyPerson {
//           id Int @id @default(autoincrement())

//           @@schema("second")
//         }
//     "#};

//     let expected = expect![[r#"
//         model FooBar {
//           id Int @id @default(autoincrement())

//           @@map("HappyPerson")
//           @@schema("first")
//         }

//         model HappyPerson {
//           id Int @id @default(autoincrement())

//           @@schema("second")
//         }
//     "#]];

//     api.expect_re_introspected_datamodel(input, expected).await;

//     let expected = expect![[r#"
//         *** WARNING ***

//         These models were enriched with `@@map` information taken from the previous Prisma schema:
//           - "FooBar"
//     "#]];

//     api.expect_re_introspect_warnings(input, expected).await;

//     Ok(())
// }

// #[test_connector(tags(Mysql), preview_features("multiSchema"), namespaces("first", "second"))]
// async fn multiple_schemas_w_duplicate_models_are_reintrospected_never_renamed(api: &mut TestApi) -> TestResult {
//     let schema_name = "first";
//     let other_name = "second";
//     let setup = formatdoc! {
//         r#"
//             CREATE SCHEMA "{schema_name}";
//             CREATE TABLE "{schema_name}"."HappyPerson" (id SERIAL PRIMARY KEY);

//             CREATE SCHEMA "{other_name}";
//             CREATE TABLE "{other_name}"."HappyPerson" (id SERIAL PRIMARY KEY);

//         "#
//     };

//     api.raw_cmd(&setup).await;

//     let input = indoc! {r#"
//         model HappyPerson {
//           id Int @id @default(autoincrement())

//           @@schema("first")
//         }
//     "#};

//     let expected = expect![[r#"
//         model HappyPerson {
//           id Int @id @default(autoincrement())

//           @@schema("first")
//         }

//         model second_HappyPerson {
//           id Int @id @default(autoincrement())

//           @@map("HappyPerson")
//           @@schema("second")
//         }
//     "#]];

//     api.expect_re_introspected_datamodel(input, expected).await;

//     let expected = expect![[r#"
//         *** WARNING ***

//         These items were renamed due to their names being duplicates in the Prisma schema:
//           - Type: "model", name: "second_HappyPerson"
//     "#]];

//     api.expect_re_introspect_warnings(input, expected).await;

//     Ok(())
// }

// #[test_connector(tags(Mysql), preview_features("multiSchema"), namespaces("first", "second"))]
// async fn multiple_schemas_w_duplicate_enums_are_reintrospected(api: &mut TestApi) -> TestResult {
//     let schema_name = "first";
//     let other_name = "second";
//     let setup = formatdoc! {
//         r#"
//             CREATE SCHEMA "{schema_name}";
//             CREATE TYPE "{schema_name}"."HappyMood" AS ENUM ('veryHappy');

//             CREATE SCHEMA "{other_name}";
//             CREATE TYPE "{other_name}"."HappyMood" AS ENUM ('veryHappy');
//         "#
//     };

//     api.raw_cmd(&setup).await;

//     let input = indoc! {r#"
//         enum RenamedMood {
//           veryHappy

//           @@map("HappyMood")
//           @@schema("first")
//         }
//     "#};

//     let expected = expect![[r#"
//         enum RenamedMood {
//           veryHappy

//           @@map("HappyMood")
//           @@schema("first")
//         }

//         enum HappyMood {
//           veryHappy

//           @@schema("second")
//         }
//     "#]];

//     api.expect_re_introspected_datamodel(input, expected).await;

//     let expected = expect![[r#"
//         *** WARNING ***

//         These enums were enriched with `@@map` information taken from the previous Prisma schema:
//           - "RenamedMood"
//     "#]];

//     api.expect_re_introspect_warnings(input, expected).await;

//     Ok(())
// }

// #[test_connector(tags(Mysql))]
// async fn multiple_schemas_w_enums_without_schemas_are_not_introspected(api: &mut TestApi) -> TestResult {
//     let schema_name = api.schema_name();
//     let other_name = "second";
//     let create_type = format!("CREATE TYPE `{schema_name}`.`HappyMood` AS ENUM ('happy')",);

//     api.database().raw_cmd(&create_type).await?;

//     let create_schema = format!("CREATE Schema `{other_name}`",);
//     let create_type = format!("CREATE TYPE `{other_name}`.`SadMood` AS ENUM ('sad')",);

//     api.database().raw_cmd(&create_schema).await?;
//     api.database().raw_cmd(&create_type).await?;

//     let expected = expect![[r#"
//         enum HappyMood {
//           happy
//         }
//     "#]];

//     let result = api.introspect_dml().await?;
//     expected.assert_eq(&result);

//     Ok(())
// }

// #[test_connector(tags(Mysql), preview_features("multiSchema"), namespaces("first", "second_schema"))]
// async fn same_table_name_with_relation_in_two_schemas(api: &mut TestApi) -> TestResult {
//     let sql = r#"
//         CREATE SCHEMA "first";
//         CREATE SCHEMA "second_schema";
//         CREATE TABLE "first"."tbl" ( id SERIAL PRIMARY KEY );
//         CREATE TABLE "second_schema"."tbl" ( id SERIAL PRIMARY KEY, fst INT REFERENCES "first"."tbl"("id") );
//     "#;

//     api.raw_cmd(sql).await;

//     let expected = expect![[r#"
//         generator client {
//           provider        = "prisma-client-js"
//           previewFeatures = ["multiSchema"]
//         }

//         datasource db {
//           provider = "mysql"
//           url      = "env(TEST_DATABASE_URL)"
//           schemas  = ["first", "second_schema"]
//         }

//         model first_tbl {
//           id  Int                 @id @default(autoincrement())
//           tbl second_schema_tbl[]

//           @@map("tbl")
//           @@schema("first")
//         }

//         model second_schema_tbl {
//           id  Int        @id @default(autoincrement())
//           fst Int?
//           tbl first_tbl? @relation(fields: [fst], references: [id], onDelete: NoAction, onUpdate: NoAction)

//           @@map("tbl")
//           @@schema("second_schema")
//         }
//     "#]];

//     api.expect_datamodel(&expected).await;

//     Ok(())
// }
