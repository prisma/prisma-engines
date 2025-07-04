use sql_introspection_tests::{test_api::*, TestResult};

// In MySQL all namespaces aka schemas are global to the whole MySQL server as "CREATE DATABASE" and "CREATE SCHEMA" are aliases.
// Hence we cannot rely on test isolation by creating fresh databases and then having multiple schemas in them as we do for Postgres.
// => We need to use unique names for the schemas by adding a suffix like `_m1` to the schema name.

#[test_connector(tags(Mysql))]
async fn multiple_schemas_without_schema_property_are_not_introspected(api: &mut TestApi) -> TestResult {
    let schema_name = api.schema_name();
    let other_name = "not_introspected_1";
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

#[test_connector(tags(Mysql), preview_features("multiSchema"), namespaces("first_m1", "second_m1"))]
async fn multiple_schemas_w_tables_are_introspected(api: &mut TestApi) -> TestResult {
    let schema_name = "first_m1";
    let other_name = "second_m1";
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

#[test_connector(tags(Mysql), preview_features("multiSchema"), namespaces("first_m2", "second_m2"))]
async fn multiple_schemas_w_tables_are_reintrospected(api: &mut TestApi) -> TestResult {
    let schema_name = "first_m2";
    let other_name = "second_m2";
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
          @@schema("first_m2")
        }

        model B {
          id   Int     @id
          data String? @db.Text

          @@index([data(length: 128)], map: "B_idx")
          @@schema("second_m2")
        }
    "#]];

    api.expect_re_introspected_datamodel(&input, expected).await;

    Ok(())
}

#[test_connector(tags(Mysql), preview_features("multiSchema"), namespaces("first_m3", "second_m3"))]
async fn multiple_schemas_w_duplicate_table_names_are_introspected(api: &mut TestApi) -> TestResult {
    let schema_name = "first_m3";
    let other_name = "second_m3";
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
          schemas  = ["first_m3", "second_m3"]
        }

        model first_m3_A {
          id Int @id

          @@map("A")
          @@schema("first_m3")
        }

        model second_m3_A {
          id Int @id

          @@map("A")
          @@schema("second_m3")
        }
    "#]];

    api.expect_datamodel(&expected).await;

    let expected = expect![[r#"
        *** WARNING ***

        These items were renamed due to their names being duplicates in the Prisma schema:
          - Type: "model", name: "first_m3_A"
          - Type: "model", name: "second_m3_A"
    "#]];

    api.expect_warnings(&expected).await;

    Ok(())
}

#[test_connector(tags(Mysql), preview_features("multiSchema"), namespaces("first_m4", "second_m4"))]
async fn multiple_schemas_w_duplicate_sanitized_table_names_are_introspected(api: &mut TestApi) -> TestResult {
    let schema_name = "first_m4";
    let other_name = "second_m4";
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
          schemas  = ["first_m4", "second_m4"]
        }

        model first_m4_2A {
          id Int @id

          @@map("2A")
          @@schema("first_m4")
        }

        model second_m4_1A {
          id Int @id

          @@map("1A")
          @@schema("second_m4")
        }
    "#]];

    api.expect_datamodel(&expected).await;

    let expected = expect![[r#"
        *** WARNING ***

        These items were renamed due to their names being duplicates in the Prisma schema:
          - Type: "model", name: "first_m4_2A"
          - Type: "model", name: "second_m4_1A"
    "#]];

    api.expect_warnings(&expected).await;

    Ok(())
}

#[test_connector(tags(Mysql), preview_features("multiSchema"), namespaces("first_m5", "second_m5"))]
async fn multiple_schemas_w_cross_schema_are_introspected(api: &mut TestApi) -> TestResult {
    let schema_name = "first_m5";
    let other_name = "second_m5";
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

          @@schema("first_m5")
        }

        model B {
          id Int  @id
          fk Int?
          A  A?   @relation(fields: [fk], references: [id], onDelete: NoAction, onUpdate: NoAction, map: "B_ibfk_1")

          @@index([fk], map: "fk")
          @@schema("second_m5")
        }
    "#]];

    let result = api.introspect_dml().await?;
    expected.assert_eq(&result);

    Ok(())
}

#[test_connector(tags(Mysql), preview_features("multiSchema"), namespaces("first_m6", "second_m6"))]
async fn multiple_schemas_w_cross_schema_are_reintrospected(api: &mut TestApi) -> TestResult {
    let schema_name = "first_m6";
    let other_name = "second_m6";
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

          @@schema("first_m6")
        }

        model B {
          id Int  @id
          fk Int?
          A  A?   @relation(fields: [fk], references: [id], onDelete: NoAction, onUpdate: NoAction)

          @@schema("first_m6")
        }
    "#};

    let expected = expect![[r#"
        model A {
          id Int @id
          B  B[]

          @@schema("first_m6")
        }

        model B {
          id Int  @id
          fk Int?
          A  A?   @relation(fields: [fk], references: [id], onDelete: NoAction, onUpdate: NoAction, map: "B_ibfk_1")

          @@index([fk], map: "fk")
          @@schema("second_m6")
        }
    "#]];

    api.expect_re_introspected_datamodel(input, expected).await;

    Ok(())
}

#[test_connector(tags(Mysql), preview_features("multiSchema"), namespaces("first_m7", "second_m7"))]
async fn multiple_schemas_w_cross_schema_fks_w_duplicate_names_are_introspected(api: &mut TestApi) -> TestResult {
    let schema_name = "first_m7";
    let other_name = "second_m7";
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
        model first_m7_A {
          id Int           @id
          A  second_m7_A[]

          @@map("A")
          @@schema("first_m7")
        }

        model second_m7_A {
          id Int         @id
          fk Int?
          A  first_m7_A? @relation(fields: [fk], references: [id], onDelete: NoAction, onUpdate: NoAction, map: "A_ibfk_1")

          @@index([fk], map: "fk")
          @@map("A")
          @@schema("second_m7")
        }
    "#]];

    let result = api.introspect_dml().await?;
    expected.assert_eq(&result);

    Ok(())
}

#[test_connector(tags(Mysql), preview_features("multiSchema"), namespaces("first_m8", "second_m8"))]
async fn multiple_schemas_w_enums_are_introspected(api: &mut TestApi) -> TestResult {
    let schema_name = "first_m8";
    let other_name = "second_m8";
    let sql = format! {
        r#"
            CREATE SCHEMA `{schema_name}`;
            CREATE TABLE `{schema_name}`.`HappyPerson` (mood ENUM ('happy') PRIMARY KEY);

            CREATE SCHEMA `{other_name}`;
            CREATE TABLE `{other_name}`.`SadPerson` (mood ENUM ('sad') PRIMARY KEY);
        "#,
    };

    api.raw_cmd(&sql).await;

    let expected = expect![[r#"
        generator client {
          provider        = "prisma-client-js"
          previewFeatures = ["multiSchema"]
        }

        datasource db {
          provider = "mysql"
          url      = "env(TEST_DATABASE_URL)"
          schemas  = ["first_m8", "second_m8"]
        }

        model HappyPerson {
          mood HappyPerson_mood @id

          @@schema("first_m8")
        }

        model SadPerson {
          mood SadPerson_mood @id

          @@schema("second_m8")
        }

        enum HappyPerson_mood {
          happy

          @@schema("first_m8")
        }

        enum SadPerson_mood {
          sad

          @@schema("second_m8")
        }
    "#]];

    api.expect_datamodel(&expected).await;
    Ok(())
}

#[test_connector(tags(Mysql), preview_features("multiSchema"), namespaces("first_m9", "second_m9"))]
async fn multiple_schemas_w_duplicate_enums_are_introspected(api: &mut TestApi) -> TestResult {
    let schema_name = "first_m9";
    let other_name = "second_m9";
    let setup = formatdoc! {
        r#"
            CREATE SCHEMA `{schema_name}`;
            CREATE TABLE `{schema_name}`.`HappyPerson` (mood ENUM ('happy') PRIMARY KEY);

            CREATE SCHEMA `{other_name}`;
            CREATE TABLE `{other_name}`.`HappyPerson` (mood ENUM ('very_happy') PRIMARY KEY);

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
          schemas  = ["first_m9", "second_m9"]
        }

        model first_m9_HappyPerson {
          mood first_m9_HappyPerson_mood @id

          @@map("HappyPerson")
          @@schema("first_m9")
        }

        model second_m9_HappyPerson {
          mood second_m9_HappyPerson_mood @id

          @@map("HappyPerson")
          @@schema("second_m9")
        }

        enum first_m9_HappyPerson_mood {
          happy

          @@map("HappyPerson_mood")
          @@schema("first_m9")
        }

        enum second_m9_HappyPerson_mood {
          very_happy

          @@map("HappyPerson_mood")
          @@schema("second_m9")
        }
    "#]];

    api.expect_datamodel(&expected).await;

    let expected = expect![[r#"
        *** WARNING ***

        These items were renamed due to their names being duplicates in the Prisma schema:
          - Type: "enum", name: "first_m9_HappyPerson_mood"
          - Type: "enum", name: "second_m9_HappyPerson_mood"
          - Type: "model", name: "first_m9_HappyPerson"
          - Type: "model", name: "second_m9_HappyPerson"
    "#]];

    api.expect_warnings(&expected).await;

    Ok(())
}

#[test_connector(tags(Mysql), preview_features("multiSchema"), namespaces("first_m10", "second_m10"))]
async fn multiple_schemas_w_duplicate_models_are_reintrospected(api: &mut TestApi) -> TestResult {
    let schema_name = "first_m10";
    let other_name = "second_m10";
    let setup = formatdoc! {
        r#"
            CREATE SCHEMA `{schema_name}`;
            CREATE TABLE `{schema_name}`.`HappyPerson` (id INT PRIMARY KEY);

            CREATE SCHEMA `{other_name}`;
            CREATE TABLE `{other_name}`.`HappyPerson` (id INT PRIMARY KEY);

        "#
    };

    api.raw_cmd(&setup).await;

    let input = indoc! {r#"
        model FooBar {
          id Int @id

          @@map("HappyPerson")
          @@schema("first_m10")
        }

        model HappyPerson {
          id Int @id

          @@schema("second_m10")
        }
    "#};

    let expected = expect![[r#"
        model FooBar {
          id Int @id

          @@map("HappyPerson")
          @@schema("first_m10")
        }

        model HappyPerson {
          id Int @id

          @@schema("second_m10")
        }
    "#]];

    api.expect_re_introspected_datamodel(input, expected).await;

    let expected = expect![[r#"
        *** WARNING ***

        These models were enriched with `@@map` information taken from the previous Prisma schema:
          - "FooBar"
    "#]];

    api.expect_re_introspect_warnings(input, expected).await;

    Ok(())
}

#[test_connector(tags(Mysql), preview_features("multiSchema"), namespaces("first_m11", "second_m11"))]
async fn multiple_schemas_w_duplicate_models_are_reintrospected_never_renamed(api: &mut TestApi) -> TestResult {
    let schema_name = "first_m11";
    let other_name = "second_m11";
    let setup = formatdoc! {
        r#"
            CREATE SCHEMA `{schema_name}`;
            CREATE TABLE `{schema_name}`.`HappyPerson` (id INT PRIMARY KEY);

            CREATE SCHEMA `{other_name}`;
            CREATE TABLE `{other_name}`.`HappyPerson` (id INT PRIMARY KEY);

        "#
    };

    api.raw_cmd(&setup).await;

    let input = indoc! {r#"
        model HappyPerson {
          id Int @id

          @@schema("first_m11")
        }
    "#};

    let expected = expect![[r#"
        model HappyPerson {
          id Int @id

          @@schema("first_m11")
        }

        model second_m11_HappyPerson {
          id Int @id

          @@map("HappyPerson")
          @@schema("second_m11")
        }
    "#]];

    api.expect_re_introspected_datamodel(input, expected).await;

    let expected = expect![[r#"
        *** WARNING ***

        These items were renamed due to their names being duplicates in the Prisma schema:
          - Type: "model", name: "second_m11_HappyPerson"
    "#]];

    api.expect_re_introspect_warnings(input, expected).await;

    Ok(())
}

#[test_connector(tags(Mysql), preview_features("multiSchema"), namespaces("first_m12", "second_m12"))]
async fn multiple_schemas_w_duplicate_enums_are_reintrospected(api: &mut TestApi) -> TestResult {
    let schema_name = "first_m12";
    let other_name = "second_m12";
    let setup = formatdoc! {
        r#"
            CREATE SCHEMA `{schema_name}`;
            CREATE TABLE `{schema_name}`.`HappyPerson` (mood ENUM ('happy') PRIMARY KEY);

            CREATE SCHEMA `{other_name}`;
            CREATE TABLE `{other_name}`.`HappyPerson` (mood ENUM ('very_happy') PRIMARY KEY);
        "#
    };

    api.raw_cmd(&setup).await;

    let input = indoc! {r#"
        model HappyPerson {
          mood RenamedMood @id

          @@schema("first_m12")
        }

        enum RenamedMood {
          happy

          @@map("HappyPerson_mood")
          @@schema("first_m12")
        }
    "#};

    let expected = expect![[r#"
        model HappyPerson {
          mood RenamedMood @id

          @@schema("first_m12")
        }

        model second_m12_HappyPerson {
          mood HappyPerson_mood @id

          @@map("HappyPerson")
          @@schema("second_m12")
        }

        enum RenamedMood {
          happy

          @@map("HappyPerson_mood")
          @@schema("first_m12")
        }

        enum HappyPerson_mood {
          very_happy

          @@schema("second_m12")
        }
    "#]];

    api.expect_re_introspected_datamodel(input, expected).await;

    let expected = expect![[r#"
        *** WARNING ***

        These enums were enriched with `@@map` information taken from the previous Prisma schema:
          - "RenamedMood"

        These items were renamed due to their names being duplicates in the Prisma schema:
          - Type: "model", name: "second_m12_HappyPerson"
    "#]];

    api.expect_re_introspect_warnings(input, expected).await;

    Ok(())
}

#[test_connector(tags(Mysql))]
async fn multiple_schemas_w_enums_without_schemas_are_not_introspected(api: &mut TestApi) -> TestResult {
    let schema_name = api.schema_name();
    let other_name = "not_introspected_2";
    let create_type = format!("CREATE TABLE `{schema_name}`.`HappyPerson` (mood ENUM ('happy') PRIMARY KEY)",);

    api.database().raw_cmd(&create_type).await?;

    let drop_schema = format!("DROP Schema IF EXISTS `{other_name}`",);
    let create_schema = format!("CREATE Schema `{other_name}`",);
    let create_type = format!("CREATE TABLE `{other_name}`.`SadPerson` (mood ENUM ('sad') PRIMARY KEY)",);

    api.database().raw_cmd(&drop_schema).await?;
    api.database().raw_cmd(&create_schema).await?;
    api.database().raw_cmd(&create_type).await?;

    let expected = expect![[r#"
        model HappyPerson {
          mood HappyPerson_mood @id
        }

        enum HappyPerson_mood {
          happy
        }
    "#]];

    let result = api.introspect_dml().await?;
    expected.assert_eq(&result);

    Ok(())
}

#[test_connector(tags(Mysql), preview_features("multiSchema"), namespaces("first_m13", "second_m13"))]
async fn same_table_name_with_relation_in_two_schemas(api: &mut TestApi) -> TestResult {
    let sql = r#"
        CREATE SCHEMA `first_m13`;
        CREATE SCHEMA `second_m13`;
        CREATE TABLE `first_m13`.`tbl` ( id INT PRIMARY KEY );
        CREATE TABLE `second_m13`.`tbl` ( id INT PRIMARY KEY, fst INT, FOREIGN KEY (fst) REFERENCES `first_m13`.`tbl`(`id`) );
    "#;

    api.raw_cmd(sql).await;

    let expected = expect![[r#"
        generator client {
          provider        = "prisma-client-js"
          previewFeatures = ["multiSchema"]
        }

        datasource db {
          provider = "mysql"
          url      = "env(TEST_DATABASE_URL)"
          schemas  = ["first_m13", "second_m13"]
        }

        model first_m13_tbl {
          id  Int              @id
          tbl second_m13_tbl[]

          @@map("tbl")
          @@schema("first_m13")
        }

        model second_m13_tbl {
          id  Int            @id
          fst Int?
          tbl first_m13_tbl? @relation(fields: [fst], references: [id], onDelete: NoAction, onUpdate: NoAction, map: "tbl_ibfk_1")

          @@index([fst], map: "fst")
          @@map("tbl")
          @@schema("second_m13")
        }
    "#]];

    api.expect_datamodel(&expected).await;

    Ok(())
}
