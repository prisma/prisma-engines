use super::test_harness::*;

#[test_each_connector]
async fn unapply_must_work(api: &TestApi) -> TestResult {
    let dm1 = r#"
        model Test {
            id String @id @default(cuid())
            field String
        }
    "#;

    api.infer_apply(&dm1).send().await?;

    let result1 = api
        .assert_schema()
        .await?
        .assert_table("Test", |table| table.assert_has_column("field"))?
        .into_schema();

    let dm2 = r#"
        model Test {
            id String @id @default(cuid())
        }
    "#;

    api.infer_apply(&dm2).send().await?;

    let result2 = api
        .assert_schema()
        .await?
        .assert_table("Test", |table| table.assert_does_not_have_column("field"))?
        .into_schema();

    api.unapply_migration().send().await?;
    api.assert_schema().await?.assert_equals(&result1)?;

    // reapply the migration again
    api.infer_apply(&dm2).send().await?;
    api.assert_schema().await?.assert_equals(&result2).map(drop)
}
