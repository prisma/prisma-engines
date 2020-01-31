use crate::error::SqlIntrospectionError;
use quaint::{
    prelude::{ConnectionInfo, Queryable, SqlFamily},
    single::Quaint,
};
use sql_schema_describer::SqlSchemaDescriberBackend;
use std::sync::Arc;
use std::time::Duration;

const CONNECTION_TIMEOUT: Duration = Duration::from_secs(10);

pub async fn load_describer(
    url: &str,
) -> Result<(Box<dyn SqlSchemaDescriberBackend>, ConnectionInfo), SqlIntrospectionError> {
    let wrapper_fut = async {
        let connection = Quaint::new(&url).await?;
        connection.query_raw("SELECT 1", &[]).await?;
        Result::Ok::<_, SqlIntrospectionError>(connection)
    };

    let wrapper = match tokio::time::timeout(CONNECTION_TIMEOUT, wrapper_fut).await {
        Ok(result) => result?,
        Err(_elapsed) => return Err(SqlIntrospectionError::ConnectTimeout),
    };

    let connection_info = wrapper.connection_info().to_owned();

    let describer: Box<dyn SqlSchemaDescriberBackend> = match connection_info.sql_family() {
        SqlFamily::Postgres => Box::new(sql_schema_describer::postgres::SqlSchemaDescriber::new(Arc::new(
            wrapper,
        ))),
        SqlFamily::Mysql => Box::new(sql_schema_describer::mysql::SqlSchemaDescriber::new(Arc::new(wrapper))),
        SqlFamily::Sqlite => Box::new(sql_schema_describer::sqlite::SqlSchemaDescriber::new(Arc::new(wrapper))),
    };

    Ok((describer, connection_info))
}
