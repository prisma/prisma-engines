use connector_interface::error::ConnectorError;
use connector_interface::Connector as _;
use mongodb_query_connector::Connector;

#[tokio::main]
async fn main() -> Result<(), ConnectorError> {
    let db_uri = "mongodb://localhost:27017/";
    let connector = Connector::new(db_uri).await?;
    let _conn = connector.get_connection().await?;
    // conn.get_single_record().await?;
    todo!();
}
