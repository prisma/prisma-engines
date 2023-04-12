use query_engine_tests::*;

/// Asserts common basics for composite type writes.
#[test_suite(schema(schema), only(Postgres))]
mod black_box_endpoint_polling {
    fn schema() -> String {
        let schema = indoc! {
            r#"model Person {
              #id(id, Int, @id)
             }"#
        };

        schema.to_owned()
    }

    #[connector_test]
    async fn test_metrics(r: Runner) -> TestResult<()> {
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

        insta::assert_snapshot!(metrics, @r###"# HELP prisma_client_queries_total Total number of Prisma Client queries executed
# TYPE prisma_client_queries_total counter
prisma_client_queries_total 0

# HELP prisma_datasource_queries_total Total number of Datasource Queries executed
# TYPE prisma_datasource_queries_total counter
prisma_datasource_queries_total 0

# HELP prisma_pool_connections_open Number of currently open Pool Connections
# TYPE prisma_pool_connections_open counter
prisma_pool_connections_open 1

# HELP prisma_client_queries_active Number of currently active Prisma Client queries
# TYPE prisma_client_queries_active gauge
prisma_client_queries_active 0

# HELP prisma_client_queries_wait Number of Prisma Client queries currently waiting for a connection
# TYPE prisma_client_queries_wait gauge
prisma_client_queries_wait 0

# HELP prisma_pool_connections_busy Number of currently busy Pool Connections (executing a database query)
# TYPE prisma_pool_connections_busy gauge
prisma_pool_connections_busy 0

# HELP prisma_pool_connections_idle Number of currently unused Pool Connections (waiting for the next pool query to run)
# TYPE prisma_pool_connections_idle gauge
prisma_pool_connections_idle 21

# HELP prisma_pool_connections_opened_total Total number of Pool Connections opened
# TYPE prisma_pool_connections_opened_total gauge
prisma_pool_connections_opened_total 1"###);

        Ok(())
    }
}
