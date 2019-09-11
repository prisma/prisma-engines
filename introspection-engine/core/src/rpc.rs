use introspection_connector::IntrospectionConnector;
use jsonrpc_core::*;
use jsonrpc_derive::rpc;
use jsonrpc_stdio_server::ServerBuilder;

pub struct RpcApi {}
impl RpcApi {
    pub fn start() {
        let mut io_handler = IoHandler::new();
        io_handler.extend_with(RpcImpl {}.to_delegate());

        let server = ServerBuilder::new(io_handler);
        server.build();
    }
}

#[rpc]
pub trait Rpc {
    #[rpc(name = "listDatabases")]
    fn list_databases(&self, url: UrlInput) -> Result<Vec<String>>;

    #[rpc(name = "getDatabaseMetadata")]
    fn get_database_metadata(&self, url: UrlInput) -> Result<DatabaseMetadata>;

    #[rpc(name = "introspect")]
    fn introspect(&self, url: UrlInput) -> Result<String>;
}

struct RpcImpl {}

impl Rpc for RpcImpl {
    fn list_databases(&self, url: UrlInput) -> Result<Vec<String>> {
        Ok(vec!["db1".to_string(), "db2".to_string()])
    }

    fn get_database_metadata(&self, url: UrlInput) -> Result<DatabaseMetadata> {
        Ok(DatabaseMetadata {
            model_count: 10,
            size_in_bytes: 1234,
        })
    }

    fn introspect(&self, url: UrlInput) -> Result<String> {
        Ok("".to_string())
    }
}

#[derive(Serialize, Deserialize)]
struct DatabaseMetadata {
    model_count: usize,
    size_in_bytes: usize,
}

#[derive(Serialize, Deserialize)]
struct UrlInput {
    url: String,
}
