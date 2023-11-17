use crate::helpers::*;
use query_engine_tests::*;

/// Asserts common basics for composite type writes.
#[test_suite(schema(schema))]
mod smoke_tests {
    use regex::Regex;
    fn schema() -> String {
        let schema = indoc! {
            r#"model Person {
              #id(id, Int, @id)
             }"#
        };

        schema.to_owned()
    }

    fn assert_value_in_range(metrics: &str, metric: &str, low: f64, high: f64) {
        let regex = Regex::new(format!(r"{metric}\s+([+-]?\d+(\.\d+)?)").as_str()).unwrap();
        match regex.captures(metrics) {
            Some(capture) => {
                let value = capture.get(1).unwrap().as_str().parse::<f64>().unwrap();
                assert!(
                    value >= low && value <= high,
                    "expected {} value of {} to be between {} and {}",
                    metric,
                    value,
                    low,
                    high
                );
            }
            None => panic!("Metric {} not found in metrics text", metric),
        }
    }

    #[connector_test]
    #[rustfmt::skip]
    async fn expected_metrics_rendered(r: Runner) -> TestResult<()> {
        let mut qe_cmd = query_engine_cmd(r.prisma_dml(), "57582");
        qe_cmd.arg("--enable-metrics");
        qe_cmd.env("PRISMA_ENGINE_PROTOCOL", "json");

        with_child_process(&mut qe_cmd, async move {
            let client = reqwest::Client::new();

            let res = client
                .post("http://0.0.0.0:57582/")
                .body(
                    r#"
                    {
                        "action": "findMany",
                        "modelName": "Person",
                        "query": {
                            "arguments": {
                            },
                            "selection": {
                                "$scalars": true
                            }
                        }
                    }
                    "#,
                )
                .send()
                .await
                .unwrap();

            assert_eq!(res.status(), 200);

            let metrics = client
                .get("http://0.0.0.0:57582/metrics")
                .send()
                .await
                .unwrap()
                .text()
                .await
                .unwrap();
            
            // I would have loved to use insta in here and check the snapshot but the order of the metrics is not guaranteed
            // And I opted for the manual checking of invariant data that provided enough confidence instead

            // counters
            assert_eq!(metrics.matches("# HELP prisma_client_queries_total The total number of Prisma Client queries executed").count(), 1);
            assert_eq!(metrics.matches("# TYPE prisma_client_queries_total counter").count(), 1);
            assert_eq!(metrics.matches("prisma_client_queries_total 1").count(), 1);
            

            assert_eq!(metrics.matches("# HELP prisma_datasource_queries_total The total number of datasource queries executed").count(), 1);
            assert_eq!(metrics.matches("# TYPE prisma_datasource_queries_total counter").count(), 1);

            assert_eq!(metrics.matches("# HELP prisma_pool_connections_closed_total The total number of pool connections closed").count(), 1);
            assert_eq!(metrics.matches("# TYPE prisma_pool_connections_closed_total counter").count(), 1);

            assert_eq!(metrics.matches("# HELP prisma_pool_connections_opened_total The total number of pool connections opened").count(), 1);
            assert_eq!(metrics.matches("# TYPE prisma_pool_connections_opened_total counter").count(), 1);

            // gauges
            assert_eq!(metrics.matches("# HELP prisma_client_queries_active The number of currently active Prisma Client queries").count(), 1);
            assert_eq!(metrics.matches("# TYPE prisma_client_queries_active gauge").count(), 1);

            assert_eq!(metrics.matches("# HELP prisma_client_queries_wait The number of datasource queries currently waiting for a free connection").count(), 1);
            assert_eq!(metrics.matches("# TYPE prisma_client_queries_wait gauge").count(), 1);

            assert_eq!(metrics.matches("# HELP prisma_pool_connections_busy The number of pool connections currently executing datasource queries").count(), 1);
            assert_eq!(metrics.matches("# TYPE prisma_pool_connections_busy gauge").count(), 1);
            assert_value_in_range(&metrics, "prisma_pool_connections_busy", 0f64, 1f64);

            assert_eq!(metrics.matches("# HELP prisma_pool_connections_idle The number of pool connections that are not busy running a query").count(), 1);
            assert_eq!(metrics.matches("# TYPE prisma_pool_connections_idle gauge").count(), 1);

            assert_eq!(metrics.matches("# HELP prisma_pool_connections_open The number of pool connections currently open").count(), 1);
            assert_eq!(metrics.matches("# TYPE prisma_pool_connections_open gauge").count(), 1);
            assert_value_in_range(&metrics, "prisma_pool_connections_open", 0f64, 1f64);
            
            // histograms
            assert_eq!(metrics.matches("# HELP prisma_client_queries_duration_histogram_ms The distribution of the time Prisma Client queries took to run end to end").count(), 1);
            assert_eq!(metrics.matches("# TYPE prisma_client_queries_duration_histogram_ms histogram").count(), 1);

            assert_eq!(metrics.matches("# HELP prisma_client_queries_wait_histogram_ms The distribution of the time all datasource queries spent waiting for a free connection").count(), 1);
            assert_eq!(metrics.matches("# TYPE prisma_client_queries_wait_histogram_ms histogram").count(), 1);
            
            assert_eq!(metrics.matches("# HELP prisma_datasource_queries_duration_histogram_ms The distribution of the time datasource queries took to run").count(), 1);
            assert_eq!(metrics.matches("# TYPE prisma_datasource_queries_duration_histogram_ms histogram").count(), 1);
            
            // Check that exist as many metrics as being accepted
            let accepted_metric_count = query_engine_metrics::ACCEPT_LIST.len();
            let displayed_metric_count = metrics.matches("# TYPE").count();
            let non_prisma_metric_count = displayed_metric_count - metrics.matches("# TYPE prisma").count();
            
            assert_eq!(displayed_metric_count, accepted_metric_count);
            assert_eq!(non_prisma_metric_count, 0);
            
        }).await
    }
}
