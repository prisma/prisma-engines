use crate::SqlError;
use quaint::error::ErrorKind;
use quaint::{
    prelude::{ConnectionInfo, Queryable, SqlFamily},
    single::Quaint,
};
use sql_schema_describer::SqlSchemaDescriberBackend;
use std::time::Duration;

const CONNECTION_TIMEOUT: Duration = Duration::from_secs(10);

pub async fn load_describer(url: &str) -> Result<(Box<dyn SqlSchemaDescriberBackend>, ConnectionInfo), SqlError> {
    let wrapper_fut = async {
        let connection = Quaint::new(&url).await?;
        let version = connection.version().await?;
        Result::Ok::<_, SqlError>((connection, version))
    };

    let (connection, version) = match tokio::time::timeout(CONNECTION_TIMEOUT, wrapper_fut).await {
        Ok(result) => result?,
        Err(_elapsed) => return Err(SqlError::from(ErrorKind::ConnectTimeout)),
    };

    let connection_info = connection.connection_info().to_owned();

    let describer: Box<dyn SqlSchemaDescriberBackend> = match connection_info.sql_family() {
        SqlFamily::Postgres => {
            let is_cockroach = version.map(|version| version.contains("CockroachDB")).unwrap_or(false);
            Box::new(sql_schema_describer::postgres::SqlSchemaDescriber::new(
                connection,
                is_cockroach,
            ))
        }
        SqlFamily::Mysql => Box::new(sql_schema_describer::mysql::SqlSchemaDescriber::new(connection)),
        SqlFamily::Sqlite => Box::new(sql_schema_describer::sqlite::SqlSchemaDescriber::new(connection)),
        SqlFamily::Mssql => Box::new(sql_schema_describer::mssql::SqlSchemaDescriber::new(connection)),
    };

    Ok((describer, connection_info))
}
