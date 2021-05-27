use crate::SqlError;
use quaint::{
    prelude::{ConnectionInfo, Queryable, SqlFamily},
    single::Quaint,
};
use sql_schema_describer::{postgres::Circumstances, SqlSchemaDescriberBackend};

pub async fn load_describer(url: &str) -> Result<(Box<dyn SqlSchemaDescriberBackend>, ConnectionInfo), SqlError> {
    let connection = Quaint::new(&url).await?;
    let version = connection.version().await?;
    let connection_info = connection.connection_info().to_owned();

    let describer: Box<dyn SqlSchemaDescriberBackend> = match connection_info.sql_family() {
        SqlFamily::Postgres => {
            let mut circumstances = Default::default();

            if version.map(|version| version.contains("CockroachDB")).unwrap_or(false) {
                circumstances |= Circumstances::Cockroach;
            }

            Box::new(sql_schema_describer::postgres::SqlSchemaDescriber::new(
                connection,
                circumstances,
            ))
        }
        SqlFamily::Mysql => Box::new(sql_schema_describer::mysql::SqlSchemaDescriber::new(connection)),
        SqlFamily::Sqlite => Box::new(sql_schema_describer::sqlite::SqlSchemaDescriber::new(connection)),
        SqlFamily::Mssql => Box::new(sql_schema_describer::mssql::SqlSchemaDescriber::new(connection)),
    };

    Ok((describer, connection_info))
}
