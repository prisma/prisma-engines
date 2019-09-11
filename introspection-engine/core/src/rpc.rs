use crate::connector_loader::load_connector;
use crate::CoreResult;
use datamodel::Datamodel;
use jsonrpc_core::*;
use jsonrpc_derive::rpc;

#[rpc]
pub trait Rpc {
    #[rpc(name = "listDatabases")]
    fn list_databases(&self, url: UrlInput) -> Result<Vec<String>>;

    #[rpc(name = "getDatabaseMetadata")]
    fn get_database_metadata(&self, url: UrlInput) -> Result<DatabaseMetadata>;

    #[rpc(name = "introspect")]
    fn introspect(&self, url: UrlInput) -> Result<String>;
}

pub struct RpcImpl {}

impl Rpc for RpcImpl {
    fn list_databases(&self, url: UrlInput) -> Result<Vec<String>> {
        Ok(Self::list_databases_internal(url)?)
    }

    fn get_database_metadata(&self, url: UrlInput) -> Result<DatabaseMetadata> {
        Ok(Self::get_database_metadata_internal(url)?)
    }

    fn introspect(&self, url: UrlInput) -> Result<String> {
        let data_model = Self::introspect_internal(url)?;
        let mut result = StringWrapper::empty();
        datamodel::render_to(&mut result, &data_model).unwrap();
        Ok(result.inner)
    }
}

impl RpcImpl {
    fn introspect_internal(url: UrlInput) -> CoreResult<Datamodel> {
        let connector = load_connector(&url.url)?;
        // FIXME: parse URL correctly via a to be built lib and pass database param;
        let data_model = connector.introspect("")?;
        Ok(data_model)
    }

    fn list_databases_internal(url: UrlInput) -> CoreResult<Vec<String>> {
        let connector = load_connector(&url.url)?;
        Ok(connector.list_databases()?)
    }

    fn get_database_metadata_internal(_url: UrlInput) -> CoreResult<DatabaseMetadata> {
        Ok(DatabaseMetadata {
            model_count: 10,
            size_in_bytes: 1234,
        })
    }
}

#[derive(Serialize, Deserialize)]
pub struct DatabaseMetadata {
    model_count: usize,
    size_in_bytes: usize,
}

#[derive(Serialize, Deserialize)]
pub struct UrlInput {
    url: String,
}

struct StringWrapper {
    inner: String,
}

impl StringWrapper {
    fn empty() -> StringWrapper {
        StringWrapper { inner: "".to_string() }
    }
}

impl std::io::Write for StringWrapper {
    fn write(&mut self, buf: &[u8]) -> std::result::Result<usize, std::io::Error> {
        let as_string = String::from_utf8(buf.to_vec()).expect("ByteArray to String failed");
        self.inner.push_str(&as_string);
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::result::Result<(), std::io::Error> {
        Ok(())
    }
}
