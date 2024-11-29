use query_engine_tests::*;

#[test_suite(schema(schema))]
mod select_different_key_type {
    use indoc::indoc;
    use query_engine_tests::{Runner, TestResult};

    fn schema() -> String {
        let schema = indoc! {
            r#"
                model FloatEntity {
                    Id Float @id
                    Text String
                }

                model BigIntEntity {
                    Id BigInt @id
                    Text String
                }

                model DecimalEntity {
                    Id Decimal @id
                    Text String
                }

                model DateEntity {
                    Id DateTime @id
                    Text String
                }
            "#
        };

        schema.to_owned()
    }

    #[connector_test]
    async fn batch_of_two_distinct(runner: Runner) -> TestResult<()> {
        create_test_data(&runner).await?;

        // These pairs of queries are run as batched and non-batched to verify that the
        // batching logic returns the same results as the non-batched logic.
        // The reason this is valuable is that the batching logic, unlike regular queries
        // relies on a comparison operation implemented in our code, which is sensitive
        // to differences in types of values.
        let pairs = [
            (
                r#"query { findUniqueFloatEntity(where: { Id: 1 }){ Text }}"#,
                r#"query { findUniqueFloatEntity(where: { Id: 2 }){ Text }}"#,
            ),
            (
                r#"query { findUniqueBigIntEntity(where: { Id: 1 }){ Text }}"#,
                r#"query { findUniqueBigIntEntity(where: { Id: 2 }){ Text }}"#,
            ),
            (
                r#"query { findUniqueDecimalEntity(where: { Id: 1 }){ Text }}"#,
                r#"query { findUniqueDecimalEntity(where: { Id: 2 }){ Text }}"#,
            ),
            (
                r#"query { findUniqueDateEntity(where: { Id: "2020-01-01T00:00:00Z" }){ Text }}"#,
                r#"query { findUniqueDateEntity(where: { Id: "2020-01-02T00:00:00Z" }){ Text }}"#,
            ),
        ];

        for (query_a, query_b) in pairs {
            let batch = runner
                .batch(vec![query_a.to_owned(), query_b.to_owned()], false, None)
                .await?
                .into_data();
            let (single_a, single_b) = futures::try_join!(runner.query(query_a), runner.query(query_b),)?;

            assert_eq!(batch.len(), 2, "{batch:?}");
            assert_eq!(
                batch,
                single_a
                    .into_data()
                    .into_iter()
                    .chain(single_b.into_data())
                    .collect::<Vec<_>>()
            );
        }

        Ok(())
    }

    #[connector_test]
    async fn batch_of_two_repeated(runner: Runner) -> TestResult<()> {
        create_test_data(&runner).await?;

        // Same as above, but with just one row repeated twice in each batch.
        let pairs = [
            (
                r#"query { findUniqueFloatEntity(where: { Id: 1 }){ Text }}"#,
                r#"query { findUniqueFloatEntity(where: { Id: 1 }){ Text }}"#,
            ),
            (
                r#"query { findUniqueBigIntEntity(where: { Id: 1 }){ Text }}"#,
                r#"query { findUniqueBigIntEntity(where: { Id: 1 }){ Text }}"#,
            ),
            (
                r#"query { findUniqueDecimalEntity(where: { Id: 1 }){ Text }}"#,
                r#"query { findUniqueDecimalEntity(where: { Id: 1 }){ Text }}"#,
            ),
            (
                r#"query { findUniqueDateEntity(where: { Id: "2020-01-01T00:00:00Z" }){ Text }}"#,
                r#"query { findUniqueDateEntity(where: { Id: "2020-01-01T00:00:00Z" }){ Text }}"#,
            ),
        ];

        for (query_a, query_b) in pairs {
            let batch = runner
                .batch(vec![query_a.to_owned(), query_b.to_owned()], false, None)
                .await?
                .into_data();
            let (single_a, single_b) = futures::try_join!(runner.query(query_a), runner.query(query_b),)?;

            assert_eq!(batch.len(), 2, "{batch:?}");
            assert_eq!(
                batch,
                single_a
                    .into_data()
                    .into_iter()
                    .chain(single_b.into_data())
                    .collect::<Vec<_>>()
            );
        }

        Ok(())
    }

    async fn create_test_data(runner: &Runner) -> TestResult<()> {
        let mutations = [
            r#"mutation entity {
                createOneFloatEntity(data: { Id: 1, Text: "A" }) {
                  Text
                }
              }"#,
            r#"mutation entity {
                createOneFloatEntity(data: { Id: 2, Text: "B" }) {
                  Text
                }
              }"#,
            r#"mutation entity {
                createOneBigIntEntity(data: { Id: 1, Text: "A" }) {
                  Text
                }
              }"#,
            r#"mutation entity {
                createOneBigIntEntity(data: { Id: 2, Text: "B" }) {
                  Text
                }
            }"#,
            r#"mutation entity {
                createOneDecimalEntity(data: { Id: 1, Text: "A" }) {
                  Text
                }
            }"#,
            r#"mutation entity {
                createOneDecimalEntity(data: { Id: 2, Text: "B" }) {
                  Text
                }
            }"#,
            r#"mutation entity {
                createOneDateEntity(data: { Id: "2020-01-01T00:00:00Z", Text: "A" }) {
                  Text
                }
            }"#,
            r#"mutation entity {
                createOneDateEntity(data: { Id: "2020-01-02T00:00:00Z", Text: "B" }) {
                  Text
                }
            }"#,
        ];

        for mutation in mutations {
            runner.query(mutation).await?.assert_success();
        }

        Ok(())
    }
}
