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
        // spwan a query-engine with metrics enabled that is configured to use the same
        // DML as the one used by the test suite as provided by the schema function above.
        let path = assert_cmd::cargo::cargo_bin("query-engine");
        let mut query_engine = std::process::Command::new(path)
            .arg("--enable-metrics")
            .arg("--port")
            .arg("57582")
            .arg("-g")
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .env("PRISMA_DML", r.prisma_dml())
            .spawn()
            .expect("Cannot spawn query-engine");

        // the Cleaner is to make sure the query-engine is killed when the test is done
        // in case of any panic
        struct Cleaner<'a> {
            p: &'a mut std::process::Child,
        }
        impl<'a> Drop for Cleaner<'a> {
            fn drop(&mut self) {
                self.p.kill().expect("Failed to kill query-engine");
            }
        }
        let _cleaner = Cleaner { p: &mut query_engine };

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

        assert!(metrics.contains("prisma_client_queries_total counter"));
        assert!(metrics.contains("prisma_datasource_queries_total counter"));
        assert!(metrics.contains("prisma_pool_connections_open counter"));
        assert!(metrics.contains("prisma_client_queries_active gauge"));
        assert!(metrics.contains("prisma_client_queries_wait gauge"));
        assert!(metrics.contains("prisma_pool_connections_busy gauge"));
        assert!(metrics.contains("prisma_pool_connections_idle gauge"));
        assert!(metrics.contains("prisma_pool_connections_opened_total gauge"));

        Ok(())
    }
}
