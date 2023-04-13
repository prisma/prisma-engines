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

        with_child_process(&mut qe_cmd, async move {
            let metrics = reqwest::get("http://0.0.0.0:57582/metrics")
                .await
                .unwrap()
                .text()
                .await
                .unwrap();

            assert!(metrics.contains("prisma_client_queries_total counter"));
            assert!(metrics.contains("prisma_datasource_queries_total counter"));
            assert!(metrics.contains("prisma_pool_connections_open counter"));
            assert!(metrics.contains("prisma_client_queries_active gauge"));
            assert!(metrics.contains("prisma_client_queries_wait gauge"));
            assert!(metrics.contains("prisma_pool_connections_busy gauge"));
            assert!(metrics.contains("prisma_pool_connections_idle gauge"));
            assert!(metrics.contains("prisma_pool_connections_opened_total gauge"));
        })
        .await
    }
}
