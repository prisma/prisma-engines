use super::error::CoreResult;
use introspection_connector::IntrospectionConnector;
use sql_introspection_connector::SqlIntrospectionConnector;

pub fn load_connector(url_str: &str) -> CoreResult<Box<dyn IntrospectionConnector>> {
    Ok(Box::new(SqlIntrospectionConnector::new(&url_str)?))
}
