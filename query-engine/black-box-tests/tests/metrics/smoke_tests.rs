use crate::helpers::*;
use query_engine_tests::*;

/// Asserts common basics for composite type writes.
#[test_suite(schema(schema))]
mod smoke_tests {
    fn schema() -> String {
        let schema = indoc! {
            r#"model Person {
              #id(id, Int, @id)
             }"#
        };

        schema.to_owned()
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

            // counters
            assert_eq!(metrics.matches("prisma_client_queries_total counter").count(), 1);
            assert_eq!(metrics.matches("prisma_datasource_queries_total counter").count(), 1);
            assert_eq!(metrics.matches("prisma_pool_connections_opened_total counter").count(), 1);
            assert_eq!(metrics.matches("prisma_pool_connections_closed_total counter").count(), 1);
            // gauges
            assert_eq!(metrics.matches("prisma_pool_connections_open gauge").count(), 1);
            assert_eq!(metrics.matches("prisma_pool_connections_busy gauge").count(), 1);
            assert_eq!(metrics.matches("prisma_pool_connections_idle gauge").count(), 1);
            assert_eq!(metrics.matches("prisma_client_queries_active gauge").count(), 1);
            assert_eq!(metrics.matches("prisma_client_queries_wait gauge").count(), 1);
            // histograms
            assert_eq!(metrics.matches("prisma_client_queries_duration_histogram_ms histogram").count(), 1);
            assert_eq!(metrics.matches("prisma_client_queries_wait_histogram_ms histogram").count(), 1);
            assert_eq!(metrics.matches("prisma_datasource_queries_duration_histogram_ms histogram").count(), 1)
        }).await
    }
}
