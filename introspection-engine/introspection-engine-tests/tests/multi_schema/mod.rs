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

#[test_connector(tags(Postgres), preview_features("multiSchema"), db_schemas())]
async fn multiple_schemas_without_schema_property_but_preview_flag_are_not_introspected(api: &TestApi) -> TestResult {
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

#[test_connector(tags(Postgres), preview_features("multiSchema"), db_schemas("first", "second"))]
async fn multiple_schemas_w_duplicate_table_names_are_introspected(api: &TestApi) -> TestResult {
    let schema_name = "first";
    let other_name = "second";
    let create_schema = format!("CREATE Schema \"{schema_name}\"",);
    let create_table = format!("CREATE TABLE \"{schema_name}\".\"A\" (id Text PRIMARY KEY)",);

    api.database().raw_cmd(&create_schema).await?;
    api.database().raw_cmd(&create_table).await?;

    let create_schema = format!("CREATE Schema \"{other_name}\"",);
    let create_table = format!("CREATE TABLE \"{other_name}\".\"A\" (id Text PRIMARY KEY)",);

    api.database().raw_cmd(&create_schema).await?;
    api.database().raw_cmd(&create_table).await?;

    let expected = expect![[r#"
        model first_A {
          id String @id

          @@map("A")
          @@schema("first")
        }

        model second_A {
          id String @id

          @@map("A")
          @@schema("second")
        }
    "#]];

    let result = api.introspect_dml().await?;
    expected.assert_eq(&result);

    Ok(())
}

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

//schema validation
// empty schemas array?
//
//preview flagging
//cross schema fks

//name conflicts
// table
// enum
//invalid names
// schema
// table
// enum
// re-introspection
// table
// enum
