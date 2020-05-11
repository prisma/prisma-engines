use connector_interface::error::{ConnectorError, ErrorKind};
use connector_interface::{self, IO};

use crate::Connection;
use mongodb::Client;

/// MongoDB connector for Prisma.
#[derive(Debug)]
pub struct Connector {
    client: Client,
}

impl Connector {
    /// Create a new instance of `Connector`.
    pub async fn new(addr: &str) -> Result<Self, ConnectorError> {
        let client = Client::with_uri_str(&addr).map_err(|_| {
            let kind = ErrorKind::InvalidConnectionArguments;
            ConnectorError::from_kind(kind)
        })?;

        Ok(Self { client })
    }
}

impl connector_interface::Connector for Connector {
    fn get_connection(&self) -> IO<'_, Box<dyn connector_interface::Connection + '_>> {
        IO::new(async move {
            let client = self.client.clone();
            let conn = Connection::new(client);
            Ok(Box::new(conn) as Box<dyn connector_interface::Connection>)
        })
    }
}
