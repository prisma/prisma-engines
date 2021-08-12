use query_engine_tests::*;

#[test_suite(schema(schema))]
mod is_null_filter {
    use indoc::indoc;
    use query_engine_tests::run_query;

    fn schema() -> String {
        let schema = indoc! {
            r#"
                model A {
                    id      String @id
                    field_a String
                    b       B?

                    @@unique([field_a, id])
                }

                model B {
                    id      String @id
                    field_b String
                    a_id    String
                    a       A      @relation(fields: [field_b, a_id], references: [field_a, id])

                    @@unique([field_b, id])
                }
              "#
        };

        schema.to_owned()
    }

    #[connector_test]
    async fn vanilla(runner: Runner) -> TestResult<()> {
        runner
            .query(r#"mutation { createOneA(data: { id: "1a", field_a: "1a"}) { id }}"#)
            .await?
            .assert_success();

        runner
            .query(r#"mutation { createOneA(data: { id: "2a", field_a: "2a", b: { create: { id: "1b" } }}) { id }}"#)
            .await?
            .assert_success();

        insta::assert_snapshot!(
          run_query!(runner, r#"{ findManyA(where: { b: { is: null }}) { id }}"#),
          @r###"{"data":{"findManyA":[{"id":"1a"}]}}"###
        );

        Ok(())
    }

    async fn create_test_data(runner: &Runner) -> TestResult<()> {
        create_row(runner, r#"{ uniqueField: 1, nonUniqFieldA: "A", nonUniqFieldB: "A"}"#).await?;

        Ok(())
    }

    async fn create_row(runner: &Runner, data: &str) -> TestResult<()> {
        runner
            .query(format!("mutation {{ createOneTestModel(data: {}) {{ id }} }}", data))
            .await?;
        Ok(())
    }
}
