use indoc::indoc;
use query_engine_tests::*;

#[test_suite(schema(schema))]
mod float_in_schema {
    fn schema() -> String {
        let schema = indoc! {
            r#"
                model FloatEntity {
                    #id(Id, Float, @id)
                    Text String
                }
            "#
        };

        schema.to_owned()
    }

    #[connector_test]
    async fn batch_of_two_distinct(runner: Runner) -> TestResult<()> {
        create_test_data(&runner).await?;
        assert_consistent_with_batch(
            &runner,
            r#"query { findUniqueFloatEntity(where: { Id: 1 }){ Text }}"#,
            r#"query { findUniqueFloatEntity(where: { Id: 2 }){ Text }}"#,
        )
        .await
    }

    #[connector_test]
    async fn batch_of_two_repeated(runner: Runner) -> TestResult<()> {
        create_test_data(&runner).await?;
        assert_consistent_with_batch(
            &runner,
            r#"query { findUniqueFloatEntity(where: { Id: 1 }){ Text }}"#,
            r#"query { findUniqueFloatEntity(where: { Id: 1 }){ Text }}"#,
        )
        .await
    }

    async fn create_test_data(runner: &Runner) -> TestResult<()> {
        for mutation in [
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
        ] {
            runner.query(mutation).await?.assert_success();
        }
        Ok(())
    }
}

#[test_suite(schema(schema))]
mod bigint_in_schema {
    fn schema() -> String {
        let schema = indoc! {
            r#"
                model BigIntEntity {
                    #id(Id, BigInt, @id)
                    Text String
                }
            "#
        };

        schema.to_owned()
    }

    #[connector_test]
    async fn batch_of_two_distinct(runner: Runner) -> TestResult<()> {
        create_test_data(&runner).await?;
        assert_consistent_with_batch(
            &runner,
            r#"query { findUniqueBigIntEntity(where: { Id: 1 }){ Text }}"#,
            r#"query { findUniqueBigIntEntity(where: { Id: 2 }){ Text }}"#,
        )
        .await
    }

    #[connector_test]
    async fn batch_of_two_repeated(runner: Runner) -> TestResult<()> {
        create_test_data(&runner).await?;
        assert_consistent_with_batch(
            &runner,
            r#"query { findUniqueBigIntEntity(where: { Id: 1 }){ Text }}"#,
            r#"query { findUniqueBigIntEntity(where: { Id: 1 }){ Text }}"#,
        )
        .await
    }

    async fn create_test_data(runner: &Runner) -> TestResult<()> {
        for mutation in [
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
        ] {
            runner.query(mutation).await?.assert_success();
        }
        Ok(())
    }
}

#[test_suite(schema(schema), exclude(MongoDb))]
mod decimal_in_schema {
    fn schema() -> String {
        let schema = indoc! {
            r#"
                model DecimalEntity {
                    #id(Id, Decimal, @id)
                    Text String
                }
            "#
        };

        schema.to_owned()
    }

    #[connector_test]
    async fn batch_of_two_distinct(runner: Runner) -> TestResult<()> {
        create_test_data(&runner).await?;
        assert_consistent_with_batch(
            &runner,
            r#"query { findUniqueDecimalEntity(where: { Id: 1 }){ Text }}"#,
            r#"query { findUniqueDecimalEntity(where: { Id: 2 }){ Text }}"#,
        )
        .await
    }

    #[connector_test]
    async fn batch_of_two_repeated(runner: Runner) -> TestResult<()> {
        create_test_data(&runner).await?;
        assert_consistent_with_batch(
            &runner,
            r#"query { findUniqueDecimalEntity(where: { Id: 1 }){ Text }}"#,
            r#"query { findUniqueDecimalEntity(where: { Id: 1 }){ Text }}"#,
        )
        .await
    }

    async fn create_test_data(runner: &Runner) -> TestResult<()> {
        for mutation in [
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
        ] {
            runner.query(mutation).await?.assert_success();
        }
        Ok(())
    }
}

#[test_suite(schema(schema))]
mod datetime_in_schema {
    fn schema() -> String {
        let schema = indoc! {
            r#"
                model DateTimeEntity {
                    #id(Id, DateTime, @id)
                    Text String
                }
            "#
        };

        schema.to_owned()
    }

    #[connector_test]
    async fn batch_of_two_distinct(runner: Runner) -> TestResult<()> {
        create_test_data(&runner).await?;
        assert_consistent_with_batch(
            &runner,
            r#"query { findUniqueDateTimeEntity(where: { Id: "2020-01-01T00:00:00Z" }){ Text }}"#,
            r#"query { findUniqueDateTimeEntity(where: { Id: "2020-01-02T00:00:00Z" }){ Text }}"#,
        )
        .await
    }

    #[connector_test]
    async fn batch_of_two_repeated(runner: Runner) -> TestResult<()> {
        create_test_data(&runner).await?;
        assert_consistent_with_batch(
            &runner,
            r#"query { findUniqueDateTimeEntity(where: { Id: "2020-01-01T00:00:00Z" }){ Text }}"#,
            r#"query { findUniqueDateTimeEntity(where: { Id: "2020-01-01T00:00:00Z" }){ Text }}"#,
        )
        .await
    }

    async fn create_test_data(runner: &Runner) -> TestResult<()> {
        for mutation in [
            r#"mutation entity {
                    createOneDateTimeEntity(data: { Id: "2020-01-01T00:00:00Z", Text: "A" }) {
                      Text
                    }
                }"#,
            r#"mutation entity {
                    createOneDateTimeEntity(data: { Id: "2020-01-02T00:00:00Z", Text: "B" }) {
                      Text
                    }
                }"#,
        ] {
            runner.query(mutation).await?.assert_success();
        }
        Ok(())
    }
}

async fn assert_consistent_with_batch(runner: &Runner, query_a: &str, query_b: &str) -> TestResult<()> {
    // These pairs of queries are run as batched and non-batched to verify that the
    // batching logic returns the same results as the non-batched logic.
    // The reason this is valuable is that the batching logic, unlike regular queries
    // relies on a comparison operation implemented in our code, which is sensitive
    // to differences in types of values.

    let batch_result = runner
        .batch(vec![query_a.to_owned(), query_b.to_owned()], false, None)
        .await?;
    batch_result.assert_success();

    let (single_a, single_b) = futures::try_join!(runner.query(query_a), runner.query(query_b),)?;

    let batch = batch_result.into_data();
    assert_eq!(batch.len(), 2, "{batch:?}");
    assert_eq!(
        batch,
        single_a
            .into_data()
            .into_iter()
            .chain(single_b.into_data())
            .collect::<Vec<_>>()
    );

    Ok(())
}
