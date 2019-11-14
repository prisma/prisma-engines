mod test_harness;

use pretty_assertions::assert_eq;
use test_harness::*;

#[test_each_connector]
async fn unapply_must_work(api: &TestApi) {
    let dm1 = r#"
            model Test {
                id String @id @default(cuid())
                field String
            }
        "#;

    let result1 = api.infer_and_apply(&dm1).await.sql_schema;
    assert!(result1.table_bang("Test").column("field").is_some());

    let dm2 = r#"
            model Test {
                id String @id @default(cuid())
            }
        "#;

    let result2 = api.infer_and_apply(&dm2).await.sql_schema;
    assert!(result2.table_bang("Test").column("field").is_none());

    let result3 = api.unapply_migration().await.sql_schema;
    assert_eq!(result1, result3);

    // reapply the migration again
    let result4 = api.infer_and_apply(&dm2).await.sql_schema;
    assert_eq!(result2, result4);
}
