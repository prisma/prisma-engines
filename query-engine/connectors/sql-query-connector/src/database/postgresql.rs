use super::connection::SqlConnection;
use crate::{query_builder::ManyRelatedRecordsWithRowNumber, FromSource, SqlError};
use connector_interface::{Connection, Connector, IO};
use datamodel::Source;
use quaint::pool::Quaint;

pub struct PostgreSql {
    pool: Quaint,
}

impl FromSource for PostgreSql {
    fn from_source(source: &dyn Source) -> crate::Result<Self> {
        Ok(PostgreSql { pool: Quaint::new(&source.url().value)? })
    }
}

impl Connector for PostgreSql {
    fn get_connection<'a>(&'a self) -> IO<Box<dyn Connection + 'a>> {
        IO::new(async move {
            let conn = self.pool.check_out().await.map_err(SqlError::from)?;
            let conn = SqlConnection::<_, ManyRelatedRecordsWithRowNumber>::new(conn);

            Ok(Box::new(conn) as Box<dyn Connection>)
        })
    }
}
