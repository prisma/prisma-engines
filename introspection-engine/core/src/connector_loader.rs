use super::error::CoreResult;
use introspection_connector::IntrospectionConnector;
use sql_introspection_connector::SqlIntrospectionConnector;

pub async fn load_connector(url_str: &str) -> CoreResult<Box<dyn IntrospectionConnector>> {
    let connector: Box<dyn IntrospectionConnector> = Box::new(SqlIntrospectionConnector::new(&url_str).await?);
    Ok(connector)
}
