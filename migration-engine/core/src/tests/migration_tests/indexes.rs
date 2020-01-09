use super::super::test_harness::*;

#[test_each_connector]
async fn index_on_compound_relation_fields_must_work(api: &TestApi) -> TestResult {
    let dm = r#"
        model User {
            id String @id
            email String
            name String

            @@unique([email, name])
        }

        model Post {
            id String @id
            author User @relation(references: [email, name])

            @@index([author])
        }
    "#;

    api.infer_apply(dm).send().await?;

    api.assert_schema()
        .await?
        .assert_table("Post", |table| {
            table
                .assert_has_column("username")?
                .assert_index_on_columns(&["a", "b"], |idx| Ok(idx))
        })
        .map(drop)
}
