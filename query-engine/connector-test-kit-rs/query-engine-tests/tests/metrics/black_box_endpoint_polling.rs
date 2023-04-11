use query_engine_tests::*;

/// Asserts common basics for composite type writes.
#[test_suite(schema(schema), only(Postgres))]
mod black_box_endpoint_polling {
    fn schema() -> String {
        let schema = indoc! {
            r#"model Person {
              #id(id, Int, @id)
              name String   @unique
              born DateTime
             }"#
        };

        schema.to_owned()
    }

    #[connector_test]
    async fn test_metrics(r: Runner) -> TestResult<()> {
        // spwan a query-engine with metrics enabled that is configured to use the same
        // DML as the one used by the test suite as provided by the schema function above.
        let path = assert_cmd::cargo::cargo_bin("query-engine");
        let mut qe = std::process::Command::new(path)
            .arg("--enable-metrics")
            .arg("--port")
            .arg("57582")
            .arg("-g")
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .env("PRISMA_DML", r.prisma_dml())
            .spawn()
            .expect("Cannot spawn query-engine");

        // wait for the query-engine to start
        std::thread::sleep(std::time::Duration::from_secs(1));

        let http = reqwest::Client::new();

        let metrics = http
            .get("http://0.0.0.0:57582/metrics")
            .send()
            .await
            .unwrap()
            .text()
            .await
            .unwrap();

        // FIXME: this is not using the translation layer added
        // Perform some queries to see this being changed.

        assert!(metrics.contains("prisma_client_queries_total"));
        assert!(metrics.contains("prisma_datasource_queries_total"));
        assert!(metrics.contains("prisma_pool_connections_open"));
        assert!(metrics.contains("prisma_client_queries_active"));
        assert!(metrics.contains("prisma_client_queries_wait"));
        assert!(metrics.contains("prisma_pool_connections_busy"));
        assert!(metrics.contains("prisma_pool_connections_idle"));
        assert!(metrics.contains("prisma_pool_connections_opened_total"));

        qe.kill().expect("Failed to kill query-engine");

        Ok(())
    }
}
