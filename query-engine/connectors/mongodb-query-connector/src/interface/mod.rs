use async_trait::async_trait;
use connector_interface::Connector;
use mongodb::Client;

pub struct MongoDb {
    client: Client,
}

#[async_trait]
impl Connector for MongoDb {
    async fn get_connection(&self) -> connector_interface::Result<Box<dyn connector_interface::Connection>> {
        self.client
    }

    fn name(&self) -> String {
        "mongodb".to_owned()
    }
}
