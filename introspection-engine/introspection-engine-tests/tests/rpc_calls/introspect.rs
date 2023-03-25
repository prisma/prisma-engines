use barrel::types;
use expect_test::expect;
use introspection_engine_tests::{test_api::*, BarrelMigrationExecutor};
use std::process::{Command, Output};
use test_macros::test_connector;

// TODO: "CARGO_BIN_EXE_introspection-engine" is not found
fn introspection_engine_bin_path() -> &'static str {
    env!("CARGO_BIN_EXE_introspection-engine")
}

// TODO: I'd expect the test to fail, but it currently doesn't run as "CARGO_BIN_EXE_introspection-engine" is not found.
#[test_connector(tags(Sqlite))]
async fn introspect_e2e_fail_when_missing_db(api: &TestApi) -> TestResult {
    use std::io::{BufRead, BufReader, Write as _};
    let tmpdir = tempfile::tempdir().unwrap();
    let schema = r#"
        datasource db {
            provider = "sqlite"
            url = "file:missing.db"
        }

    "#;

    // this creates an empty "missing.db" file to disk
    let mut process = Command::new(introspection_engine_bin_path())
        .env("RUST_LOG", "INFO")
        .env(
            "TEST_DB_URL",
            format!("file:{}/dev.db", tmpdir.path().to_string_lossy()),
        )
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()
        .unwrap();
    let stdin = process.stdin.as_mut().unwrap();
    let mut stdout = BufReader::new(process.stdout.as_mut().unwrap());

    for iteration in 0..2 {
        let msg = serde_json::to_string(&serde_json::json!({
            "jsonrpc": "2.0",
            "method": "introspect",
            "id": iteration,
            "params": {
                "schema": schema,
                "force": true,
                "compositeTypeDepth": 5,
            }
        }))
        .unwrap();
        stdin.write_all(msg.as_bytes()).unwrap();
        stdin.write_all(b"\n").unwrap();

        let mut response = String::new();
        stdout.read_line(&mut response).unwrap();

        dbg!("response", &response);

        // TODO: I expect a failure rather than a response
        assert!(response.starts_with(r##"{"jsonrpc":"2.0","result":{"datamodel":"datasource db {\n  provider = \"sqlite\"\n  url      = "file:missing.db"\n}\n","version":"NonPrisma","warnings":[]},"##));
    }
    Ok(())
}
