use expect_test::expect;
use schema_core::{ExtensionTypeConfig, RpcApi};
use std::sync::Arc;
use test_macros::test_connector;
use test_setup::*;

struct TestApi {
    _args: TestApiArgs,
    api: RpcApi,
    rt: tokio::runtime::Runtime,
}

impl TestApi {
    fn new(_args: TestApiArgs) -> Self {
        let host = Arc::new(schema_core::schema_connector::EmptyHost);
        let rt = tokio::runtime::Runtime::new().unwrap();
        TestApi {
            _args,
            api: RpcApi::new(None, None, host, Arc::new(ExtensionTypeConfig::default())),
            rt,
        }
    }

    fn send_request(&mut self, request: &str) -> Option<String> {
        self.rt.block_on(self.api.io_handler().handle_request(request))
    }
}

#[test_connector(tags(Sqlite))]
fn test_can_connect_to_database(mut api: TestApi) {
    let tempdir = tempfile::tempdir().unwrap();
    let sqlite_schema_path = tempdir.path().join("test.sqlite");
    std::fs::File::create(&sqlite_schema_path).unwrap();
    let url = format!("file:{}", sqlite_schema_path.to_string_lossy().into_owned());
    let request = r#"
        {"jsonrpc":"2.0","id":1,"method":"ensureConnectionValidity","params":{"datasource":{"tag":"ConnectionString", "url": "theurl"}}}
    "#.replace("theurl", &url);

    let response = api.send_request(&request).unwrap();

    let expected = expect![[r#"{"jsonrpc":"2.0","result":{},"id":1}"#]];

    expected.assert_eq(&response);
}

#[test_connector(tags(Sqlite))]
fn test_create_database(mut api: TestApi) {
    let tempdir = tempfile::tempdir().unwrap();
    let url = format!(
        "file:{}",
        tempdir.path().join("test.sqlite").to_string_lossy().into_owned()
    );
    let request = r#"
        {"jsonrpc":"2.0","id":1,"method":"createDatabase","params":{"datasource":{"tag":"ConnectionString", "url": "theurl"}}}
    "#.replace("theurl", &url);

    let response = api.send_request(&request).unwrap();
    assert!(response.starts_with(r#"{"jsonrpc":"2.0","result""#)); // success
}
