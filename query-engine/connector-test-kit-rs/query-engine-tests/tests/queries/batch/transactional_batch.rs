use query_engine_tests::*;

#[test_suite(schema(schema))]
mod transactional {
    use indoc::indoc;
    use query_engine_tests::run_query;

    fn schema() -> String {
        let schema = indoc! {
            r#"
                model ModelA {
                    #id(id, Int, @id)
                    b_id Int?
                    b ModelB? @relation(fields: [b_id], references: [id])
                }

                model ModelB {
                    #id(id, Int, @id)
                    a  ModelA?
                }

                model ModelC {
                    #id(id, Int, @id)
                }
            "#
        };

        schema.to_owned()
    }

    #[connector_test(exclude(SqlServer))]
    async fn two_success(runner: Runner) -> TestResult<()> {
        let queries = vec![
            r#"mutation { createOneModelA(data: { id: 1 }) { id }}"#.to_string(),
            r#"mutation { createOneModelA(data: { id: 2 }) { id }}"#.to_string(),
        ];

        let batch_results = runner.batch(queries, true).await?;
        insta::assert_snapshot!(
            batch_results.to_string(),
            @r###"{"batchResult":[{"data":{"createOneModelA":{"id":1}}},{"data":{"createOneModelA":{"id":2}}}]}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn one_success_one_fail(runner: Runner) -> TestResult<()> {
        let queries = vec![
            r#"mutation { createOneModelA(data: { id: 1 }) { id }}"#.to_string(),
            r#"mutation { createOneModelA(data: { id: 1 }) { id }}"#.to_string(),
        ];

        let batch_results = runner.batch(queries, true).await?;
        batch_results.assert_failure(2002, None);

        insta::assert_snapshot!(
            run_query!(&runner, r#"{ findManyModelA { id } }"#),
            @r###"{"data":{"findManyModelA":[]}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn one_query(runner: Runner) -> TestResult<()> {
        // Existing ModelA in the DB will prevent the nested ModelA creation in the batch.
        insta::assert_snapshot!(
            run_query!(&runner, r#"mutation {
                createOneModelA(data: { id: 1 }) {
                  id
                }
              }"#),
            @r###"{"data":{"createOneModelA":{"id":1}}}"###
        );

        let queries = vec![
            r#"mutation { createOneModelB(data: { id: 1, a: { create: { id: 1 } } }) { id }}"#.to_string(), // ModelB gets created before ModelA because of inlining,
        ];

        let batch_results = runner.batch(queries, true).await?;
        batch_results.assert_failure(2002, None);

        insta::assert_snapshot!(
            run_query!(&runner, r#"{ findManyModelB { id } }"#),
            @r###"{"data":{"findManyModelB":[]}}"###
        );

        Ok(())
    }

    // Only postgres for basic testing
    #[connector_test(only(Postgres))]
    async fn raw_mix(runner: Runner) -> TestResult<()> {
        // Existing ModelA in the DB will prevent the nested ModelA creation in the batch.
        insta::assert_snapshot!(
            run_query!(&runner, r#"mutation {
                createOneModelA(data: { id: 1 }) {
                  id
                }
              }"#),
            @r###"{"data":{"createOneModelA":{"id":1}}}"###
        );

        let queries = vec![
            r#"mutation { createOneModelB(data: { id: 1, a: { connect: { id: 1 } } }) { id }}"#.to_string(),
            r#"mutation { executeRaw(query: "INSERT INTO \"ModelA\" (id, b_id) VALUES(2, NULL)", parameters: "[]") }"#
                .to_string(),
            r#"mutation { queryRaw(query: "SELECT * FROM \"ModelC\"", parameters: "[]") }"#.to_string(),
        ];

        let batch_results = runner.batch(queries, true).await?;
        insta::assert_snapshot!(
            batch_results.to_string(),
            @r###"{"batchResult":[{"data":{"createOneModelB":{"id":1}}},{"data":{"executeRaw":1}},{"data":{"queryRaw":[]}}]}"###
        );

        Ok(())
    }
}
