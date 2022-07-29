use query_engine_tests::test_suite;

#[test_suite(schema(generic))]
mod metrics {
    use query_engine_tests::ConnectorVersion::{MongoDb, SqlServer, Sqlite};
    use query_engine_tests::*;
    use serde_json::Value;

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
        let total_queries = get_counter(&json, "query_total_queries");
        let total_operations = get_counter(&json, "query_total_operations");

        match runner.connector_version() {
            Sqlite => assert_eq!(total_queries, 9),
            SqlServer(_) => assert_eq!(total_queries, 15),
            MongoDb(_) => assert_eq!(total_queries, 5),
            _ => assert_eq!(total_queries, 11),
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
        let active_transactions = get_gauge(&json, "query_active_transactions");
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
        let active_transactions = get_gauge(&json, "query_active_transactions");
        assert_eq!(active_transactions, 0.0);
        Ok(())
    }

    fn get_counter(json: &Value, name: &str) -> u64 {
        let metric_value = get_metric_value(json, "counters", name);
        metric_value.as_u64().unwrap()
    }

    fn get_gauge(json: &Value, name: &str) -> f64 {
        let metric_value = get_metric_value(json, "gauges", name);
        metric_value.as_f64().unwrap()
    }

    fn get_metric_value(json: &Value, metric_type: &str, name: &str) -> serde_json::Value {
        let metrics = json.get(metric_type).unwrap().as_array().unwrap();
        let metric = metrics
            .iter()
            .find(|metric| metric.get("key").unwrap().as_str() == Some(name))
            .unwrap()
            .as_object()
            .unwrap();

        metric.get("value").unwrap().clone()
    }
}
