use connector_interface::error::{ConnectorError, ErrorKind};
use connector_interface::{self};

use crate::Connection;
use mongodb::Client;
use async_trait::async_trait;

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

#[async_trait]
impl connector_interface::Connector for Connector {
    async fn get_connection(&self) -> connector_interface::Result<Box<dyn connector_interface::Connection>> {
            let client = self.client.clone();
            let conn = Connection::new(client);
            Ok(Box::new(conn) as Box<dyn connector_interface::Connection>)
    }
}
