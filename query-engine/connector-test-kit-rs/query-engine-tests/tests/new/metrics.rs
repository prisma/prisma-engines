use query_engine_tests::test_suite;

#[test_suite(
    schema(generic),
    exclude(
        Vitess("planetscale.js", "planetscale.js.wasm"),
        Postgres("neon.js", "pg.js", "neon.js.wasm", "pg.js.wasm"),
        Sqlite("libsql.js", "libsql.js.wasm", "cfd1")
    )
)]
mod metrics {
    use query_engine_metrics::{
        PRISMA_CLIENT_QUERIES_ACTIVE, PRISMA_CLIENT_QUERIES_TOTAL, PRISMA_DATASOURCE_QUERIES_TOTAL,
    };
    use query_engine_tests::ConnectorVersion::*;
    use query_engine_tests::*;

    #[connector_test]
    async fn metrics_are_recorded(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation { createOneTestModel(data: { id: 1 }) { id }}"#),
          @r###"{"data":{"createOneTestModel":{"id":1}}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation { updateOneTestModel(where: { id: 1 }, data: { field: "updated" }) { field } }"#),
          @r###"{"data":{"updateOneTestModel":{"field":"updated"}}}"###
        );

        let json = runner.get_metrics().to_json(Default::default());
        // We cannot assert the full response it will be slightly different per database
        let total_queries = utils::metrics::get_counter(&json, PRISMA_DATASOURCE_QUERIES_TOTAL);
        let total_operations = utils::metrics::get_counter(&json, PRISMA_CLIENT_QUERIES_TOTAL);

        match runner.connector_version() {
            Sqlite(_) => assert_eq!(total_queries, 2),
            SqlServer(_) => assert_eq!(total_queries, 17),
            MongoDb(_) => assert_eq!(total_queries, 5),
            CockroachDb(_) => (), // not deterministic
            MySql(_) => assert_eq!(total_queries, 12),
            Vitess(_) => assert_eq!(total_queries, 11),
            Postgres(_) => assert_eq!(total_queries, 7),
        }

        assert_eq!(total_operations, 2);
        Ok(())
    }

    #[connector_test]
    async fn metrics_tx_do_not_go_negative(mut runner: Runner) -> TestResult<()> {
        let tx_id = runner.start_tx(5000, 5000, None).await?;
        runner.set_active_tx(tx_id.clone());

        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation { createOneTestModel(data: { id: 1 }) { id }}"#),
          @r###"{"data":{"createOneTestModel":{"id":1}}}"###
        );

        let _ = runner.commit_tx(tx_id.clone()).await?;
        let _ = runner.commit_tx(tx_id.clone()).await?;
        let _ = runner.commit_tx(tx_id.clone()).await?;
        let _ = runner.commit_tx(tx_id).await?;

        let json = runner.get_metrics().to_json(Default::default());
        let active_transactions = utils::metrics::get_gauge(&json, PRISMA_CLIENT_QUERIES_ACTIVE);
        assert_eq!(active_transactions, 0.0);

        let tx_id = runner.start_tx(5000, 5000, None).await?;
        runner.set_active_tx(tx_id.clone());

        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation { createOneTestModel(data: { id: 2 }) { id }}"#),
          @r###"{"data":{"createOneTestModel":{"id":2}}}"###
        );

        let _ = runner.rollback_tx(tx_id.clone()).await?;
        let _ = runner.rollback_tx(tx_id.clone()).await?;
        let _ = runner.rollback_tx(tx_id.clone()).await?;
        let _ = runner.rollback_tx(tx_id.clone()).await?;

        let json = runner.get_metrics().to_json(Default::default());
        let active_transactions = utils::metrics::get_gauge(&json, PRISMA_CLIENT_QUERIES_ACTIVE);
        assert_eq!(active_transactions, 0.0);
        Ok(())
    }
}
