use crate::{Circumstances, SqlError};
use enumflags2::BitFlags;
use quaint::{
    prelude::{ConnectionInfo, SqlFamily},
    single::Quaint,
};
use sql_schema_describer::{postgres::Circumstances as PostgresCircumstances, SqlSchemaDescriberBackend};

pub(crate) async fn load_describer(
    url: &str,
) -> Result<
    (
        Box<dyn SqlSchemaDescriberBackend>,
        ConnectionInfo,
        BitFlags<Circumstances>,
    ),
    SqlError,
> {
    let connection = Quaint::new(&url).await?;
    let circumstances = Circumstances::new(&connection).await?;
    let connection_info = connection.connection_info().to_owned();

    let describer: Box<dyn SqlSchemaDescriberBackend> = match connection_info.sql_family() {
        SqlFamily::Postgres => {
            let mut postgres_circumstances = BitFlags::empty();

            if circumstances.contains(Circumstances::Cockroach) {
                postgres_circumstances |= PostgresCircumstances::Cockroach;
            }

            Box::new(sql_schema_describer::postgres::SqlSchemaDescriber::new(
                connection,
                postgres_circumstances,
            ))
        }
        SqlFamily::Mysql => Box::new(sql_schema_describer::mysql::SqlSchemaDescriber::new(connection)),
        SqlFamily::Sqlite => Box::new(sql_schema_describer::sqlite::SqlSchemaDescriber::new(connection)),
        SqlFamily::Mssql => Box::new(sql_schema_describer::mssql::SqlSchemaDescriber::new(connection)),
    };

    Ok((describer, connection_info, circumstances))
}
