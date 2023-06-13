use async_trait::async_trait;

// TODO: design the error type and make it an enum with variants that would generalise
// to Node-API, WASM etc.
pub type Error = Box<dyn std::error::Error>;

pub type Result<T> = std::result::Result<T, Error>;

#[async_trait]
pub trait Driver: Send + Sync {
    async fn query_raw(&self, sql: String) -> Result<ResultSet>;
    async fn execute_raw(&self, sql: String) -> Result<u32>;
    async fn version(&self) -> Result<Option<String>>;
    async fn close(&self) -> Result<()>;
    fn is_healthy(&self) -> Result<bool>;
}

#[derive(Debug)]
pub struct ResultSet {
    pub columns: Vec<String>,

    // TODO: support any JS-serializable type, not just String.
    pub rows: Vec<Vec<String>>,
}
