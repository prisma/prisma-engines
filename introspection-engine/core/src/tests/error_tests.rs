use crate::rpc::{RpcImpl, UrlInput};
use pretty_assertions::assert_eq;
use serde_json::json;
use test_setup::*;
use url::Url;

#[tokio::test]
async fn unreachable_database_must_return_a_proper_error_on_mysql() {
    let mut url: Url = mysql_url().parse().unwrap();

    url.set_port(Some(8787)).unwrap();

    let error = RpcImpl::introspect_internal(UrlInput { url: url.to_string() })
        .await
        .unwrap_err();

    let port = url.port().unwrap();
    let host = url.host().unwrap().to_string();

    let json_error = serde_json::to_value(error.data.unwrap()).unwrap();
    let expected = json!({
        "message": format!("Can't reach database server at `{host}`:`{port}`\n\nPlease make sure your database server is running at `{host}`:`{port}`.", host = host, port = port),
        "meta": {
            "database_host": host,
            "database_port": port,
        },
        "error_code": "P1001"
    });

    assert_eq!(json_error, expected);
}
