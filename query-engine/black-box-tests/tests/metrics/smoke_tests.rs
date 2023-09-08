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
            assert!(metrics.contains("prisma_client_queries_total counter"));
            assert!(metrics.contains("prisma_datasource_queries_total counter"));
            assert!(metrics.contains("prisma_pool_connections_opened_total counter"));
            assert!(metrics.contains("prisma_pool_connections_closed_total counter"));
            // gauges
            assert!(metrics.contains("prisma_pool_connections_open gauge"));
            assert!(metrics.contains("prisma_pool_connections_busy gauge"));
            assert!(metrics.contains("prisma_pool_connections_idle gauge"));
            assert!(metrics.contains("prisma_client_queries_active gauge"));
            assert!(metrics.contains("prisma_client_queries_wait gauge"));
            // histograms
            assert!(metrics.contains("prisma_client_queries_duration_histogram_ms histogram"));
            assert!(metrics.contains("prisma_client_queries_wait_histogram_ms histogram"));
            assert!(metrics.contains("prisma_datasource_queries_duration_histogram_ms histogram"));
        })
        .await
    }
}
