use migration_engine_tests::sql::*;
use prisma_value::PrismaValue;
use quaint::{prelude::Insert, prelude::Queryable, Value};
use sql_schema_describer::DefaultValue;

#[test_each_connector(tags("sqlite"))]
async fn changing_a_column_from_optional_to_required_must_warn(api: &TestApi) -> TestResult {
    let dm = r#"
        model Test {
            id String @id @default(cuid())
            age Int?
        }
    "#;

    api.infer_apply(&dm).send().await?.assert_green()?;

    let insert = Insert::multi_into(api.render_table_name("Test"), &["id", "age"])
        .values(("a", 12))
        .values(("b", 22))
        .values(("c", Value::Integer(None)));

    api.database().query(insert.into()).await.unwrap();

    let dm2 = r#"
        model Test {
            id String @id @default(cuid())
            age Int @default(30)
        }
    "#;

    // TODO: this should not warn, since this specific migration is safe.
    api.infer_apply(&dm2)
        .force(Some(true))
        .send()
        .await?
        .assert_executable()?
        .assert_no_error()?
        .assert_warnings(&["You are about to alter the column `age` on the `Test` table, which still contains 2 non-null values. The data in that column could be lost.".into()])?;

    api.assert_schema().await?.assert_table("Test", |table| {
        table.assert_column("age", |column| {
            column
                .assert_default(Some(DefaultValue::VALUE(PrismaValue::Int(30))))?
                .assert_is_required()
        })
    })?;

    // Check that no data was lost.
    {
        let data = api.dump_table("Test").await?;
        assert_eq!(data.len(), 3);
        let ages: Vec<Option<i64>> = data.into_iter().map(|row| row.get("age").unwrap().as_i64()).collect();

        // TODO: this is NOT what users would expect (it's a consequence of the stepped migration
        // process), we should have a more specific warning for this.
        assert_eq!(ages, &[Some(12), Some(22), Some(30)]);
    }

    Ok(())
}
