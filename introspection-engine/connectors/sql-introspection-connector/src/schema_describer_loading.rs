use crate::SqlError;
use quaint::error::ErrorKind;
use quaint::{
    prelude::{ConnectionInfo, Queryable, SqlFamily},
    single::Quaint,
};
use sql_schema_describer::SqlSchemaDescriberBackend;
use std::sync::Arc;
use std::time::Duration;

const CONNECTION_TIMEOUT: Duration = Duration::from_secs(10);

pub async fn load_describer(url: &str) -> Result<(Box<dyn SqlSchemaDescriberBackend>, ConnectionInfo), SqlError> {
    let wrapper_fut = async {
        let connection = Quaint::new(&url).await?;
        connection.query_raw("SELECT 1", &[]).await?;
        Result::Ok::<_, SqlError>(connection)
    };

    let wrapper = match tokio::time::timeout(CONNECTION_TIMEOUT, wrapper_fut).await {
        Ok(result) => result?,
        Err(_elapsed) => return Err(SqlError::from(ErrorKind::ConnectTimeout("Tokio timer".into()))),
    };

    let connection_info = wrapper.connection_info().to_owned();

    let describer: Box<dyn SqlSchemaDescriberBackend> = match connection_info.sql_family() {
        SqlFamily::Postgres => Box::new(sql_schema_describer::postgres::SqlSchemaDescriber::new(Arc::new(
            wrapper,
        ))),
        SqlFamily::Mysql => Box::new(sql_schema_describer::mysql::SqlSchemaDescriber::new(Arc::new(wrapper))),
        SqlFamily::Sqlite => Box::new(sql_schema_describer::sqlite::SqlSchemaDescriber::new(Arc::new(wrapper))),
        SqlFamily::Mssql => todo!("Greetings from Redmond"),
    };

    Ok((describer, connection_info))
}
