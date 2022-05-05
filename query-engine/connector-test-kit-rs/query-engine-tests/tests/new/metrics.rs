use query_engine_tests::test_suite;
use serde_json::json;

#[test_suite(schema(generic))]
mod metrics {
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

        let expected = json!({
        "counters":[],
        "gauges": [
            {
                "key": "pool.active_connections",
                "labels": {},
                "value": 2.0,
                "description": "The number of active connections in use."
            }, {
                "key": "pool.idle_connections",
                "labels": {},
                "value": 19.0,
                "description": "The number of connections that are not being used"
            },{
                "key": "pool.wait_count",
                "labels": {},
                "value": 0.0,
                "description": "The number of workers waiting for a connection."
            }],
            "histograms":[{
                "key":"pool.wait_duration",
                "labels":{},
                "value":[[0.0,0],[1.0,2],[2.0,2],[5.0,2],[10.0,2],[20.0,2],[50.0,2],[100.0,2],[200.0,2],[500.0,2],[1000.0,2],[2000.0,2],[5000.0,2]],
                "description":"The wait time for a worker to get a connection."
            }]
        });

        let json = runner.get_metrics().to_json();
        assert_eq!(expected, json);

        Ok(())
    }
}
