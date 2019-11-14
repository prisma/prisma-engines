use crate::{SqlIntrospectionError, SqlIntrospectionResult};
use sql_schema_describer::SqlSchemaDescriberBackend;
use std::sync::Arc;

pub fn load_describer(url_str: &str) -> SqlIntrospectionResult<Box<dyn SqlSchemaDescriberBackend>> {
    if url_str.starts_with("postgresql://") {
        let wrapper = sql_connection::GenericSqlConnection::from_database_str(url_str, None)?;

        Ok(Box::new(sql_schema_describer::postgres::SqlSchemaDescriber::new(
            Arc::new(wrapper),
        )))
    } else if url_str.starts_with("mysql://") {
        let wrapper = sql_connection::GenericSqlConnection::from_database_str(url_str, None)?;

        Ok(Box::new(sql_schema_describer::mysql::SqlSchemaDescriber::new(
            Arc::new(wrapper),
        )))
    } else if url_str.starts_with("file:") {
        let wrapper = sql_connection::GenericSqlConnection::from_database_str(url_str, Some("introspection-engine"))?;
        Ok(Box::new(sql_schema_describer::sqlite::SqlSchemaDescriber::new(
            Arc::new(wrapper),
        )))
    } else {
        Err(SqlIntrospectionError::InvalidUrl {
            message: format!("Could not load connector for the provided url: {}", url_str),
        })
    }
}
