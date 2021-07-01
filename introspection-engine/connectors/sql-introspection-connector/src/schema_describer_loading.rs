use quaint::prelude::{ConnectionInfo, Queryable, SqlFamily};
use sql_schema_describer::{postgres::Circumstances, SqlSchemaDescriberBackend};

#[tracing::instrument(skip(connection))]
pub async fn load_describer<'a>(
    connection: &'a dyn Queryable,
    connection_info: &ConnectionInfo,
) -> Result<Box<dyn SqlSchemaDescriberBackend + 'a>, crate::SqlError> {
    let version = connection.version().await?;

    Ok(match connection_info.sql_family() {
        SqlFamily::Postgres => {
            let mut circumstances = Default::default();

            if version.map(|version| version.contains("CockroachDB")).unwrap_or(false) {
                circumstances |= Circumstances::Cockroach;
            }

            Box::new(sql_schema_describer::postgres::SqlSchemaDescriber::new(
                connection,
                circumstances,
            )) as Box<dyn SqlSchemaDescriberBackend>
        }
        SqlFamily::Mysql => Box::new(sql_schema_describer::mysql::SqlSchemaDescriber::new(connection)),
        SqlFamily::Sqlite => Box::new(sql_schema_describer::sqlite::SqlSchemaDescriber::new(connection)),
        SqlFamily::Mssql => Box::new(sql_schema_describer::mssql::SqlSchemaDescriber::new(connection)),
    })
}
