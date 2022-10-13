use introspection_engine_tests::{test_api::*, TestResult};
use test_macros::test_connector;

#[test_connector(tags(Postgres))]
async fn multiple_schemas_without_schema_property_are_not_introspected(api: &TestApi) -> TestResult {
    let schema_name = api.schema_name();
    let other_name = "second";
    let create_table = format!("CREATE TABLE \"{schema_name}\".\"A\" (id Text PRIMARY KEY, data Text)",);
    let create_primary = format!("CREATE INDEX \"A_idx\" ON \"{schema_name}\".\"A\" (\"data\")",);

    api.database().raw_cmd(&create_table).await?;
    api.database().raw_cmd(&create_primary).await?;

    let create_schema = format!("CREATE Schema \"{other_name}\"",);
    let create_table = format!("CREATE TABLE \"{other_name}\".\"B\" (id Text PRIMARY KEY, data Text)",);
    let create_primary = format!("CREATE INDEX \"B_idx\" ON \"{other_name}\".\"B\" (\"data\")",);

    api.database().raw_cmd(&create_schema).await?;
    api.database().raw_cmd(&create_table).await?;
    api.database().raw_cmd(&create_primary).await?;

    let expected = expect![[r#"
        model A {
          id   String  @id
          data String?

          @@index([data], map: "A_idx")
        }
    "#]];

    let result = api.introspect_dml().await?;
    expected.assert_eq(&result);

    Ok(())
}

#[test_connector(tags(Postgres), preview_features("multiSchema"), db_schemas("first", "second"))]
async fn multiple_schemas_w_tables_are_introspected(api: &TestApi) -> TestResult {
    let schema_name = "first";
    let other_name = "second";
    let create_schema = format!("CREATE Schema \"{schema_name}\"",);
    let create_table = format!("CREATE TABLE \"{schema_name}\".\"A\" (id Text PRIMARY KEY, data Text)",);
    let create_primary = format!("CREATE INDEX \"A_idx\" ON \"{schema_name}\".\"A\" (\"data\")",);

    api.database().raw_cmd(&create_schema).await?;
    api.database().raw_cmd(&create_table).await?;
    api.database().raw_cmd(&create_primary).await?;

    let create_schema = format!("CREATE Schema \"{other_name}\"",);
    let create_table = format!("CREATE TABLE \"{other_name}\".\"B\" (id Text PRIMARY KEY, data Text)",);
    let create_primary = format!("CREATE INDEX \"B_idx\" ON \"{other_name}\".\"B\" (\"data\")",);

    api.database().raw_cmd(&create_schema).await?;
    api.database().raw_cmd(&create_table).await?;
    api.database().raw_cmd(&create_primary).await?;

    let expected = expect![[r#"
        model A {
          id   String  @id
          data String?

          @@index([data], map: "A_idx")
          @@schema("first")
        }

        model B {
          id   String  @id
          data String?

          @@index([data], map: "B_idx")
          @@schema("second")
        }
    "#]];

    let result = api.introspect_dml().await?;
    expected.assert_eq(&result);

    Ok(())
}

//TODO(matthias) this is not working yet, but this is what it would look like if done ;-)
// #[test_connector(tags(Postgres), preview_features("multiSchema"), db_schemas("first", "second"))]
// async fn multiple_schemas_w_duplicate_table_names_are_introspected(api: &TestApi) -> TestResult {
//     let schema_name = "first";
//     let other_name = "second";
//     let create_schema = format!("CREATE Schema \"{schema_name}\"",);
//     let create_table = format!("CREATE TABLE \"{schema_name}\".\"A\" (id Text PRIMARY KEY)",);
//
//     api.database().raw_cmd(&create_schema).await?;
//     api.database().raw_cmd(&create_table).await?;
//
//     let create_schema = format!("CREATE Schema \"{other_name}\"",);
//     let create_table = format!("CREATE TABLE \"{other_name}\".\"A\" (id Text PRIMARY KEY)",);
//
//     api.database().raw_cmd(&create_schema).await?;
//     api.database().raw_cmd(&create_table).await?;
//
//     let expected = expect![[r#"
//         model first_A {
//           id String @id
//
//           @@map("A")
//           @@schema("first")
//         }
//
//         model second_A {
//           id String @id
//
//           @@map("A")
//           @@schema("second")
//         }
//     "#]];
//
//     let result = api.introspect_dml().await?;
//     expected.assert_eq(&result);
//
//     Ok(())
// }

#[test_connector(tags(Postgres), preview_features("multiSchema"), db_schemas("first", "second"))]
async fn multiple_schemas_w_cross_schema_are_introspected(api: &TestApi) -> TestResult {
    let schema_name = "first";
    let other_name = "second";
    let create_schema = format!("CREATE Schema \"{schema_name}\"",);
    let create_table = format!("CREATE TABLE \"{schema_name}\".\"A\" (id Text PRIMARY KEY)",);
    //Todo
    api.database().raw_cmd(&create_schema).await?;
    api.database().raw_cmd(&create_table).await?;

    let create_schema = format!("CREATE Schema \"{other_name}\"",);
    let create_table = format!(
        "CREATE TABLE \"{other_name}\".\"B\" (id Text PRIMARY KEY, fk Text References \"{schema_name}\".\"A\"(\"id\"))",
    );

    api.database().raw_cmd(&create_schema).await?;
    api.database().raw_cmd(&create_table).await?;

    let expected = expect![[r#"
        model A {
          id String @id
          B  B[]

          @@schema("first")
        }

        model B {
          id String  @id
          fk String?
          A  A?      @relation(fields: [fk], references: [id], onDelete: NoAction, onUpdate: NoAction)

          @@schema("second")
        }
    "#]];

    let result = api.introspect_dml().await?;
    expected.assert_eq(&result);

    Ok(())
}

//TODO(matthias) this is not working yet, but this is what it would look like if done ;-)
// #[test_connector(tags(Postgres), preview_features("multiSchema"), db_schemas("first", "second"))]
// async fn multiple_schemas_w_cross_schema_fks_w_duplicate_names_are_introspected(api: &TestApi) -> TestResult {
//     let schema_name = "first";
//     let other_name = "second";
//     let create_schema = format!("CREATE Schema \"{schema_name}\"",);
//     let create_table = format!("CREATE TABLE \"{schema_name}\".\"A\" (id Text PRIMARY KEY)",);
//     //Todo
//     api.database().raw_cmd(&create_schema).await?;
//     api.database().raw_cmd(&create_table).await?;
//
//     let create_schema = format!("CREATE Schema \"{other_name}\"",);
//     let create_table = format!(
//         "CREATE TABLE \"{other_name}\".\"A\" (id Text PRIMARY KEY, fk Text References \"{schema_name}\".\"A\"(\"id\"))",
//     );
//
//     api.database().raw_cmd(&create_schema).await?;
//     api.database().raw_cmd(&create_table).await?;
//
//     let expected = expect![[r#"
//         model first_A {
//           id String @id
//           second_A  second_A[]
//
//           @@map("A")
//           @@schema("first")
//         }
//
//         model second_A {
//           id String  @id
//           fk String?
//           first_A  first_A?      @relation(fields: [fk], references: [id], onDelete: NoAction, onUpdate: NoAction)
//
//           @@map("A")
//           @@schema("second")
//         }
//     "#]];
//
//     let result = api.introspect_dml().await?;
//     expected.assert_eq(&result);
//
//     Ok(())
// }

#[test_connector(tags(Postgres), preview_features("multiSchema"), db_schemas("first", "second"))]
async fn multiple_schemas_w_enums_are_introspected(api: &TestApi) -> TestResult {
    let schema_name = "first";
    let other_name = "second";
    let create_schema = format!("CREATE Schema \"{schema_name}\"",);
    let create_type = format!("CREATE TYPE \"{schema_name}\".\"HappyMood\" AS ENUM ('happy')",);

    api.database().raw_cmd(&create_schema).await?;
    api.database().raw_cmd(&create_type).await?;

    let create_schema = format!("CREATE Schema \"{other_name}\"",);
    let create_type = format!("CREATE TYPE \"{other_name}\".\"SadMood\" AS ENUM ('sad')",);

    api.database().raw_cmd(&create_schema).await?;
    api.database().raw_cmd(&create_type).await?;

    let expected = expect![[r#"
        enum HappyMood {
          happy

          @@schema("first")
        }

        enum SadMood {
          sad

          @@schema("second")
        }
    "#]];

    let result = api.introspect_dml().await?;
    expected.assert_eq(&result);

    Ok(())
}

// Todo(matthias) not passing yet due to us not retrieving and passing around the schema information of an enum
// when it is used in a column type. We only pass the name currently which might not be unique
// therefore the renaming logic for name clashes is not working for point of use yet
// #[test_connector(tags(Postgres), preview_features("multiSchema"), db_schemas("first", "second"))]
// async fn multiple_schemas_w_duplicate_enums_are_introspected(api: &TestApi) -> TestResult {
//     let schema_name = "first";
//     let other_name = "second";
//     let create_schema = format!("CREATE Schema \"{schema_name}\"",);
//     let create_type = format!("CREATE TYPE \"{schema_name}\".\"HappyMood\" AS ENUM ('happy')",);
//     let create_table =
//         format!("CREATE TABLE \"{schema_name}\".\"HappyPerson\" (mood \"{schema_name}\".\"HappyMood\" PRIMARY KEY)",);
//
//     api.database().raw_cmd(&create_schema).await?;
//     api.database().raw_cmd(&create_type).await?;
//     api.database().raw_cmd(&create_table).await?;
//
//     let create_schema = format!("CREATE Schema \"{other_name}\"",);
//     let create_type = format!("CREATE TYPE \"{other_name}\".\"HappyMood\" AS ENUM ('veryHappy')",);
//     let create_table =
//         format!("CREATE TABLE \"{other_name}\".\"VeryHappyPerson\" (mood \"{other_name}\".\"HappyMood\" PRIMARY KEY)",);
//
//     let create_table_2 =
//         format!("CREATE TABLE \"{other_name}\".\"HappyPerson\" (mood \"{schema_name}\".\"HappyMood\" PRIMARY KEY)",);
//
//     api.database().raw_cmd(&create_schema).await?;
//     api.database().raw_cmd(&create_type).await?;
//     api.database().raw_cmd(&create_table).await?;
//     api.database().raw_cmd(&create_table_2).await?;
//
//     let expected = expect![[r#"
//         model HappyPerson {
//           mood first_HappyMood @id
//
//           @@schema("first")
//         }
//
//         model HappyPerson {
//           mood first_HappyMood @id
//
//           @@schema("second")
//         }
//
//         model VeryHappyPerson {
//           mood second_HappyMood @id
//
//           @@schema("second")
//         }
//
//         enum first_HappyMood {
//           happy
//
//           @@map("HappyMood")
//           @@schema("first")
//         }
//
//         enum second_HappyMood {
//           veryHappy
//
//           @@map("HappyMood")
//           @@schema("second")
//         }
//     "#]];
//
//     let result = api.introspect_dml().await?;
//     expected.assert_eq(&result);
//
//     Ok(())
// }

#[test_connector(tags(Postgres))]
async fn multiple_schemas_w_enums_without_schemas_are_not_introspected(api: &TestApi) -> TestResult {
    let schema_name = api.schema_name();
    let other_name = "second";
    let create_type = format!("CREATE TYPE \"{schema_name}\".\"HappyMood\" AS ENUM ('happy')",);

    api.database().raw_cmd(&create_type).await?;

    let create_schema = format!("CREATE Schema \"{other_name}\"",);
    let create_type = format!("CREATE TYPE \"{other_name}\".\"SadMood\" AS ENUM ('sad')",);

    api.database().raw_cmd(&create_schema).await?;
    api.database().raw_cmd(&create_type).await?;

    let expected = expect![[r#"
        enum HappyMood {
          happy
        }
    "#]];

    let result = api.introspect_dml().await?;
    expected.assert_eq(&result);

    Ok(())
}

//cross schema
// fks
// enums

//Edge cases
//name conflicts
// what if the names are used somewhere???
// table
// enum
//invalid names
// schema
// table
// enum
// re-introspection
// table
// enum
