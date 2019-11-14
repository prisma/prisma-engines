use super::connection::SqlConnection;
use crate::{query_builder::ManyRelatedRecordsWithUnionAll, FromSource, SqlError};
use connector_interface::{Connection, Connector, IO};
use datamodel::Source;
use quaint::Quaint;

pub struct Mysql {
    pool: Quaint,
}

impl FromSource for Mysql {
    fn from_source(source: &dyn Source) -> crate::Result<Self> {
        Ok(Mysql { pool: Quaint::new(&source.url().value)? })
    }
}

impl Connector for Mysql {
    fn get_connection<'a>(&'a self) -> IO<Box<dyn Connection + 'a>> {
        IO::new(async move {
            let conn = self.pool.check_out().await.map_err(SqlError::from)?;
            let conn = SqlConnection::<_, ManyRelatedRecordsWithUnionAll>::new(conn);

            Ok(Box::new(conn) as Box<dyn Connection>)
        })
    }
}
