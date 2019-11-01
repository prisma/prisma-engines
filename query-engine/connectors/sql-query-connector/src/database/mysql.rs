use super::connection::SqlConnection;
use crate::{query_builder::ManyRelatedRecordsWithUnionAll, FromSource, QueryExt, SqlError};
use connector_interface::{Connection, Connector, IO};
use datamodel::Source;
use prisma_query::pool::{self, MysqlManager};
use tokio_resource_pool::{CheckOut, Pool};
use url::Url;

pub struct Mysql {
    pool: Pool<MysqlManager>,
}

impl QueryExt for CheckOut<MysqlManager> {}

impl FromSource for Mysql {
    fn from_source(source: &dyn Source) -> crate::Result<Self> {
        let url = Url::parse(&source.url().value)?;
        let pool = pool::mysql(url)?;

        Ok(Mysql { pool })
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
