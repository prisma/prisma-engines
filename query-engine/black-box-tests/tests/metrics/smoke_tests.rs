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

            println!("{}", &metrics);

            assert_eq!(metrics.matches("# HELP prisma_client_queries_total The total number of Prisma Client queries executed").count(), 1);
            assert_eq!(metrics.matches("# TYPE prisma_client_queries_total counter").count(), 1);
            assert_eq!(metrics.matches("prisma_client_queries_total 1").count(), 1);

            assert_eq!(metrics.matches("# HELP prisma_datasource_queries_total The total number of datasource queries executed").count(), 1);
            assert_eq!(metrics.matches("# TYPE prisma_datasource_queries_total counter").count(), 1);
            assert_eq!(metrics.matches("prisma_datasource_queries_total 1").count(), 1);

            assert_eq!(metrics.matches("# HELP prisma_pool_connections_closed_total The total number of pool connections closed").count(), 1);
            assert_eq!(metrics.matches("# TYPE prisma_pool_connections_closed_total counter").count(), 1);
            assert_eq!(metrics.matches("prisma_pool_connections_closed_total 0").count(), 1);

            assert_eq!(metrics.matches("# HELP prisma_pool_connections_opened_total The total number of pool connections opened").count(), 1);
            assert_eq!(metrics.matches("# TYPE prisma_pool_connections_opened_total counter").count(), 1);
            assert_eq!(metrics.matches("prisma_pool_connections_opened_total 1").count(), 1);

            assert_eq!(metrics.matches("# HELP prisma_client_queries_active The number of currently active Prisma Client queries").count(), 1);
            assert_eq!(metrics.matches("# TYPE prisma_client_queries_active gauge").count(), 1);
            assert_eq!(metrics.matches("prisma_client_queries_active 0").count(), 1);

            assert_eq!(metrics.matches("# HELP prisma_client_queries_wait The number of datasource queries currently waiting for an free connection").count(), 1);
            assert_eq!(metrics.matches("# TYPE prisma_client_queries_wait gauge").count(), 1);
            assert_eq!(metrics.matches("prisma_client_queries_wait 0").count(), 1);

            assert_eq!(metrics.matches("# HELP prisma_pool_connections_busy The number of pool connections currently executing datasource queries").count(), 1);
            assert_eq!(metrics.matches("# TYPE prisma_pool_connections_busy gauge").count(), 1);
            assert_eq!(metrics.matches("prisma_pool_connections_busy 0").count(), 1);

            assert_eq!(metrics.matches("# HELP prisma_pool_connections_idle The number of pool connections that are not busy running a query").count(), 1);
            assert_eq!(metrics.matches("# TYPE prisma_pool_connections_idle gauge").count(), 1);
            assert_eq!(metrics.matches("prisma_pool_connections_idle 21").count(), 1);

            assert_eq!(metrics.matches("# HELP prisma_pool_connections_open The number of pool connections currently open").count(), 1);
            assert_eq!(metrics.matches("# TYPE prisma_pool_connections_open gauge").count(), 1);
            assert_eq!(metrics.matches("prisma_pool_connections_open 1").count(), 1);

            assert_eq!(metrics.matches("# HELP prisma_client_queries_duration_histogram_ms The distribution of the time Prisma Client queries took to run end to end").count(), 1);
            assert_eq!(metrics.matches("# TYPE prisma_client_queries_duration_histogram_ms histogram").count(), 1);
            assert_eq!(metrics.matches("prisma_client_queries_duration_histogram_ms_bucket{le=\"0\"}").count(), 1);
            assert_eq!(metrics.matches("prisma_client_queries_duration_histogram_ms_bucket{le=\"1\"}").count(), 1);
            assert_eq!(metrics.matches("prisma_client_queries_duration_histogram_ms_bucket{le=\"5\"}").count(), 1);
            assert_eq!(metrics.matches("prisma_client_queries_duration_histogram_ms_bucket{le=\"10\"}").count(), 1);
            assert_eq!(metrics.matches("prisma_client_queries_duration_histogram_ms_bucket{le=\"50\"}").count(), 1);
            assert_eq!(metrics.matches("prisma_client_queries_duration_histogram_ms_bucket{le=\"100\"}").count(), 1);
            assert_eq!(metrics.matches("prisma_client_queries_duration_histogram_ms_bucket{le=\"500\"}").count(), 1);
            assert_eq!(metrics.matches("prisma_client_queries_duration_histogram_ms_bucket{le=\"1000\"}").count(), 1);
            assert_eq!(metrics.matches("prisma_client_queries_duration_histogram_ms_bucket{le=\"5000\"}").count(), 1);
            assert_eq!(metrics.matches("prisma_client_queries_duration_histogram_ms_bucket{le=\"50000\"}").count(), 1);
            assert_eq!(metrics.matches("prisma_client_queries_duration_histogram_ms_bucket{le=\"+Inf\"}").count(), 1);
            assert_eq!(metrics.matches("prisma_client_queries_duration_histogram_ms_sum").count(), 1);
            assert_eq!(metrics.matches("prisma_client_queries_duration_histogram_ms_count 1").count(), 1);

            assert_eq!(metrics.matches("# HELP prisma_client_queries_wait_histogram_ms The distribution of the time all datasource queries spent waiting for a free connection").count(), 1);
            assert_eq!(metrics.matches("# TYPE prisma_client_queries_wait_histogram_ms histogram").count(), 1);
            assert_eq!(metrics.matches("prisma_client_queries_wait_histogram_ms_bucket{le=\"0\"}").count(), 1);
            assert_eq!(metrics.matches("prisma_client_queries_wait_histogram_ms_bucket{le=\"1\"}").count(), 1);
            assert_eq!(metrics.matches("prisma_client_queries_wait_histogram_ms_bucket{le=\"5\"}").count(), 1);
            assert_eq!(metrics.matches("prisma_client_queries_wait_histogram_ms_bucket{le=\"10\"}").count(), 1);
            assert_eq!(metrics.matches("prisma_client_queries_wait_histogram_ms_bucket{le=\"50\"}").count(), 1);
            assert_eq!(metrics.matches("prisma_client_queries_wait_histogram_ms_bucket{le=\"100\"}").count(), 1);
            assert_eq!(metrics.matches("prisma_client_queries_wait_histogram_ms_bucket{le=\"500\"}").count(), 1);
            assert_eq!(metrics.matches("prisma_client_queries_wait_histogram_ms_bucket{le=\"1000\"}").count(), 1);
            assert_eq!(metrics.matches("prisma_client_queries_wait_histogram_ms_bucket{le=\"5000\"}").count(), 1);
            assert_eq!(metrics.matches("prisma_client_queries_wait_histogram_ms_bucket{le=\"50000\"}").count(), 1);
            assert_eq!(metrics.matches("prisma_client_queries_wait_histogram_ms_bucket{le=\"+Inf\"}").count(), 1);
            assert_eq!(metrics.matches("prisma_client_queries_wait_histogram_ms_sum").count(), 1);
            assert_eq!(metrics.matches("prisma_client_queries_wait_histogram_ms_count").count(), 1);

            assert_eq!(metrics.matches("# HELP prisma_datasource_queries_duration_histogram_ms The distribution of the time datasource queries took to run").count(), 1);
            assert_eq!(metrics.matches("# TYPE prisma_datasource_queries_duration_histogram_ms histogram").count(), 1);
            assert_eq!(metrics.matches("prisma_datasource_queries_duration_histogram_ms_bucket{le=\"0\"}").count(), 1);
            assert_eq!(metrics.matches("prisma_datasource_queries_duration_histogram_ms_bucket{le=\"1\"}").count(), 1);
            assert_eq!(metrics.matches("prisma_datasource_queries_duration_histogram_ms_bucket{le=\"5\"}").count(), 1);
            assert_eq!(metrics.matches("prisma_datasource_queries_duration_histogram_ms_bucket{le=\"10\"}").count(), 1);
            assert_eq!(metrics.matches("prisma_datasource_queries_duration_histogram_ms_bucket{le=\"50\"}").count(), 1);
            assert_eq!(metrics.matches("prisma_datasource_queries_duration_histogram_ms_bucket{le=\"100\"}").count(), 1);
            assert_eq!(metrics.matches("prisma_datasource_queries_duration_histogram_ms_bucket{le=\"500\"}").count(), 1);
            assert_eq!(metrics.matches("prisma_datasource_queries_duration_histogram_ms_bucket{le=\"1000\"}").count(), 1);
            assert_eq!(metrics.matches("prisma_datasource_queries_duration_histogram_ms_bucket{le=\"5000\"}").count(), 1);
            assert_eq!(metrics.matches("prisma_datasource_queries_duration_histogram_ms_bucket{le=\"50000\"}").count(), 1);
            assert_eq!(metrics.matches("prisma_datasource_queries_duration_histogram_ms_bucket{le=\"+Inf\"}").count(), 1);
            assert_eq!(metrics.matches("prisma_datasource_queries_duration_histogram_ms_sum").count(), 1);
            assert_eq!(metrics.matches("prisma_datasource_queries_duration_histogram_ms_count").count(), 1);
        }).await
    }
}
