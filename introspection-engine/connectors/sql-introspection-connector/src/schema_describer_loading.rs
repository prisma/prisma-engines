use crate::{SqlIntrospectionError, SqlIntrospectionResult};
use quaint::Quaint;
use sql_schema_describer::SqlSchemaDescriberBackend;
use std::sync::Arc;

pub fn load_describer(url_str: &str) -> SqlIntrospectionResult<Box<dyn SqlSchemaDescriberBackend>> {
    if url_str.starts_with("postgresql://") {
        let wrapper = Quaint::new(url_str)?;

        Ok(Box::new(sql_schema_describer::postgres::SqlSchemaDescriber::new(
            Arc::new(wrapper),
        )))
    } else if url_str.starts_with("mysql://") {
        let wrapper = Quaint::new(url_str)?;

        Ok(Box::new(sql_schema_describer::mysql::SqlSchemaDescriber::new(
            Arc::new(wrapper),
        )))
    } else if url_str.starts_with("file:") {
        let wrapper = Quaint::new(&format!("{}?db_name=introspection-engine", url_str))?;
        Ok(Box::new(sql_schema_describer::sqlite::SqlSchemaDescriber::new(
            Arc::new(wrapper),
        )))
    } else {
        Err(SqlIntrospectionError::InvalidUrl {
            message: format!("Could not load connector for the provided url: {}", url_str),
        })
    }
}
