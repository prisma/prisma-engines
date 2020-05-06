use connector_interface::error::{ConnectorError, ErrorKind};
use connector_interface::{self, Connection, Connector, IO};

use mongodb::Client;

/// MongoDB connector for Prisma.
#[derive(Debug)]
pub struct Mongodb {
    addr: String,
}

impl Mongodb {
    /// Create a new instance of `Mongodb`.
    pub fn new(addr: &str) -> Self {
        Self { addr: addr.to_string() }
    }
}

impl Connector for Mongodb {
    fn get_connection(&self) -> IO<'_, Box<dyn Connection + '_>> {
        IO::new(async move {
            let _client = Client::with_uri_str(&self.addr).map_err(|_| {
                let kind = ErrorKind::InvalidConnectionArguments;
                ConnectorError::from_kind(kind)
            })?;
            todo!();
        })
    }
}
