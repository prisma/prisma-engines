use crate::*;
use barrel::types;
use test_harness::*;

#[test_each_connector(tags("postgres"))]
async fn introspecting_a_table_enums_should_work(api: &TestApi) {
    let sql = format!("CREATE Type color as ENUM ( 'black', 'white')");
    let sql2 = format!("CREATE Type color2 as ENUM ( 'black2', 'white2')");

    api.database().execute_raw(&sql, &[]).await.unwrap();
    api.database().execute_raw(&sql2, &[]).await.unwrap();

    api.barrel()
        .execute(|migration| {
            migration.create_table("Book", |t| {
                t.add_column("id", types::primary());
                t.inject_custom("color  color Not Null");
                t.inject_custom("color2  color2 Not Null");
            });
        })
        .await;

    let dm = r#"
        model Book {
            color   color
            color2  color2
            id      Int     @default(autoincrement()) @id
        }

        enum color{
            black
            white
        }

        enum color2{
            black2
            white2
        }
    "#;

    let result = dbg!(api.introspect().await);
    let result1 = dbg!(api.introspect().await);
    let result2 = dbg!(api.introspect().await);
    let result3 = dbg!(api.introspect().await);
    let result4 = dbg!(api.introspect().await);
    custom_assert(&result, dm);
    custom_assert(&result1, dm);
    custom_assert(&result2, dm);
    custom_assert(&result3, dm);
    custom_assert(&result4, dm);
}

#[test_each_connector(tags("postgres"))]
async fn introspecting_a_table_enums_should_return_alphabetically_even_when_in_different_order(api: &TestApi) {
    let sql1 = format!("CREATE Type color as ENUM ( 'black', 'white')");
    let sql2 = format!("CREATE Type color2 as ENUM ( 'black2', 'white2')");

    api.database().execute_raw(&sql2, &[]).await.unwrap();
    api.database().execute_raw(&sql1, &[]).await.unwrap();

    api.barrel()
        .execute(|migration| {
            migration.create_table("Book", |t| {
                t.add_column("id", types::primary());
                t.inject_custom("color2  Color2 Not Null");
                t.inject_custom("color  Color Not Null");
            });
        })
        .await;

    let dm = r#"
        model Book {
            color   color
            color2  color2
            id      Int     @default(autoincrement()) @id
        }

        enum color{
            black
            white
        }

        enum color2{
            black2
            white2
        }
    "#;

    let result = dbg!(api.introspect().await);
    let result1 = dbg!(api.introspect().await);
    let result2 = dbg!(api.introspect().await);
    let result3 = dbg!(api.introspect().await);
    let result4 = dbg!(api.introspect().await);
    custom_assert(&result, dm);
    custom_assert(&result1, dm);
    custom_assert(&result2, dm);
    custom_assert(&result3, dm);
    custom_assert(&result4, dm);
}

#[test_each_connector(tags("postgres"))]
async fn introspecting_a_table_enums_array_should_work(api: &TestApi) {
    let sql = format!("CREATE Type color as ENUM ( 'black', 'white')");

    api.database().execute_raw(&sql, &[]).await.unwrap();

    api.barrel()
        .execute(|migration| {
            migration.create_table("Book", |t| {
                t.add_column("id", types::primary());
                t.inject_custom("color  color []");
            });
        })
        .await;

    let dm = r#"
        datasource pg {
              provider = "postgres"
              url = "postgresql://localhost:5432"
        }

        model Book {
            color   color[]
            id      Int     @default(autoincrement()) @id
        }

        enum color{
            black
            white
        }
    "#;

    let result = dbg!(api.introspect().await);
    custom_assert(&result, dm);
}

#[test_each_connector(tags("postgres"))]
async fn introspecting_a_table_with_enum_default_values_should_work(api: &TestApi) {
    let sql = format!("CREATE Type color as ENUM ( 'black', 'white')");

    api.database().execute_raw(&sql, &[]).await.unwrap();

    api.barrel()
        .execute(|migration| {
            migration.create_table("Book", |t| {
                t.add_column("id", types::primary());
                t.inject_custom("color  color Not Null default 'black'");
            });
        })
        .await;

    let dm = r#"
        model Book {
            color   color   @default(black)
            id      Int     @default(autoincrement()) @id
        }

        enum color{
            black
            white
        }
    "#;

    let result = dbg!(api.introspect().await);
    custom_assert(&result, dm);
}

#[test_each_connector(tags("postgres"))]
async fn introspecting_a_table_with_enum_default_values_that_look_like_booleans_should_work(api: &TestApi) {
    let sql = format!("CREATE Type Truth as ENUM ( 'true', 'false', 'rumor')");

    api.database().execute_raw(&sql, &[]).await.unwrap();

    api.barrel()
        .execute(|migration| {
            migration.create_table("News", |t| {
                t.add_column("id", types::primary());
                t.inject_custom("confirmed  truth Not Null default 'true'");
            });
        })
        .await;

    let dm = r#"
        model News {
            confirmed   truth   @default(true)
            id          Int     @default(autoincrement()) @id
        }

        enum truth{
            false
            rumor
            true
        }
    "#;

    let result = dbg!(api.introspect().await);
    custom_assert(&result, dm);
}

#[test_each_connector(tags("postgres"))]
async fn introspecting_an_enum_with_an_invalid_value_should_work(api: &TestApi) {
    let sql = format!("CREATE Type status as ENUM ( '1', 'UNDEFINED')");

    api.database().execute_raw(&sql, &[]).await.unwrap();

    api.barrel()
        .execute(|migration| {
            migration.create_table("News", |t| {
                t.add_column("id", types::primary());
                t.inject_custom("status  status Not Null default 'UNDEFINED'");
            });
        })
        .await;

    let warnings = dbg!(api.introspection_warnings().await);
    assert_eq!(&warnings, "[{\"code\":4,\"message\":\"These enum values were commented out because of invalid names. Please provide valid ones that match [a-zA-Z][a-zA-Z0-9_]*.\",\"affected\":[{\"enm\":\"status\",\"value\":\"1\"}]}]");

    let result = dbg!(api.introspect().await);
    assert_eq!(&result, "model News {\n  id     Int    @default(autoincrement()) @id\n  status status @default(UNDEFINED)\n}\n\nenum status {\n  // 1 @map(\"1\")\n  UNDEFINED\n}");
}

#[test_each_connector(tags("postgres"))]
async fn introspecting_an_enum_with_an_invalid_value_as_default_should_work(api: &TestApi) {
    let sql = format!("CREATE Type status as ENUM ( '1', 'UNDEFINED')");

    api.database().execute_raw(&sql, &[]).await.unwrap();

    api.barrel()
        .execute(|migration| {
            migration.create_table("News", |t| {
                t.add_column("id", types::primary());
                t.inject_custom("status  status Not Null default '1'");
            });
        })
        .await;

    let warnings = dbg!(api.introspection_warnings().await);
    assert_eq!(&warnings, "[{\"code\":4,\"message\":\"These enum values were commented out because of invalid names. Please provide valid ones that match [a-zA-Z][a-zA-Z0-9_]*.\",\"affected\":[{\"enm\":\"status\",\"value\":\"1\"}]}]");

    let result = dbg!(api.introspect().await);
    assert_eq!(&result, "model News {\n  id     Int    @default(autoincrement()) @id\n  status status @default(dbgenerated())\n}\n\nenum status {\n  // 1 @map(\"1\")\n  UNDEFINED\n}");
}
