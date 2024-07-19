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
                    b_id Int? @unique
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

        let batch_results = runner.batch(queries, true, None).await?;
        insta::assert_snapshot!(
            batch_results.to_string(),
            @r###"{"batchResult":[{"data":{"createOneModelA":{"id":1}}},{"data":{"createOneModelA":{"id":2}}}]}"###
        );

        Ok(())
    }

    #[connector_test(exclude(Sqlite("cfd1")))]
    // On D1, this fails with:
    //
    // ```diff
    // - {"data":{"findManyModelA":[]}}
    // + {"data":{"findManyModelA":[{"id":1}]}}
    // ```
    async fn one_success_one_fail(runner: Runner) -> TestResult<()> {
        let queries = vec![
            r#"mutation { createOneModelA(data: { id: 1 }) { id }}"#.to_string(),
            r#"mutation { createOneModelA(data: { id: 1 }) { id }}"#.to_string(),
        ];

        let batch_results = runner.batch(queries, true, None).await?;
        batch_results.assert_failure(2002, None);

        insta::assert_snapshot!(
            run_query!(&runner, r#"{ findManyModelA { id } }"#),
            @r###"{"data":{"findManyModelA":[]}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn batch_request_idx(runner: Runner) -> TestResult<()> {
        let queries = vec![
            r#"mutation { createOneModelA(data: { id: 1 }) { id }}"#.to_string(),
            r#"mutation { createOneModelA(data: { id: 1 }) { id }}"#.to_string(),
        ];

        let batch_results = runner.batch(queries, true, None).await?;
        let batch_request_idx = batch_results.errors().first().unwrap().batch_request_idx();

        assert_eq!(batch_request_idx, Some(1));

        Ok(())
    }

    #[connector_test(exclude(Sqlite("cfd1")))]
    // On D1, this fails with:
    //
    // ```diff
    // - {"data":{"findManyModelB":[]}}
    // + {"data":{"findManyModelB":[{"id":1}]}}
    // ```
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

        let batch_results = runner.batch(queries, true, None).await?;
        batch_results.assert_failure(2002, None);

        insta::assert_snapshot!(
            run_query!(&runner, r#"{ findManyModelB { id } }"#),
            @r###"{"data":{"findManyModelB":[]}}"###
        );

        Ok(())
    }

    // On PlanetScale, this fails with:
    // "Error in connector: Error querying the database: Server error: `ERROR 25001 (1568): Transaction characteristics can't be changed while a transaction is in progress'""
    #[connector_test(exclude(MongoDb, Vitess("planetscale.js", "planetscale.js.wasm")))]
    async fn valid_isolation_level(runner: Runner) -> TestResult<()> {
        let queries = vec![r#"mutation { createOneModelB(data: { id: 1 }) { id }}"#.to_string()];

        let batch_results = runner.batch(queries, true, Some("Serializable".into())).await?;

        insta::assert_snapshot!(batch_results.to_string(), @r###"{"batchResult":[{"data":{"createOneModelB":{"id":1}}}]}"###);

        Ok(())
    }

    #[connector_test(exclude(MongoDb))]
    async fn invalid_isolation_level(runner: Runner) -> TestResult<()> {
        let queries = vec![r#"mutation { createOneModelB(data: { id: 1 }) { id }}"#.to_string()];

        let batch_results = runner.batch(queries, true, Some("NotALevel".into())).await?;

        batch_results.assert_failure(2023, Some("Invalid isolation level `NotALevel`".into()));

        Ok(())
    }

    #[connector_test(only(MongoDb))]
    async fn isolation_level_mongo(runner: Runner) -> TestResult<()> {
        let queries = vec![r#"mutation { createOneModelB(data: { id: 1 }) { id }}"#.to_string()];

        let batch_results = runner.batch(queries, true, Some("Serializable".into())).await?;
        batch_results.assert_failure(
            2026,
            Some("Mongo does not support setting transaction isolation levels".into()),
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

        let batch_results = runner.batch(queries, true, None).await?;
        insta::assert_snapshot!(
            batch_results.to_string(),
            @r###"{"batchResult":[{"data":{"createOneModelB":{"id":1}}},{"data":{"executeRaw":1}},{"data":{"queryRaw":{"columns":["id"],"types":["int"],"rows":[]}}}]}"###
        );

        Ok(())
    }
}
