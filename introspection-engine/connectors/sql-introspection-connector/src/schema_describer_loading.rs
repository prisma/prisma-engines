use quaint::{error::Error as QuaintError, prelude::ConnectionInfo, Quaint};
use sql_schema_describer::SqlSchemaDescriberBackend;
use std::sync::Arc;

pub fn load_describer(url: &str) -> Result<(Box<dyn SqlSchemaDescriberBackend>, ConnectionInfo), QuaintError> {
    let wrapper = Quaint::new(&url)?;

    let connection_info = wrapper.connection_info().to_owned();
    let describer = Box::new(sql_schema_describer::postgres::SqlSchemaDescriber::new(Arc::new(
        wrapper,
    )));

    Ok((describer, connection_info))
}
