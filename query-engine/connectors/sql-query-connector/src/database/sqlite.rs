use super::connection::SqlConnection;
use crate::{query_builder::ManyRelatedRecordsWithRowNumber, FromSource, SqlError};
use connector_interface::{Connection, Connector, IO};
use datamodel::Source;
use quaint::{connector::SqliteParams, Quaint};
use std::convert::TryFrom;
use url::Url;

pub struct Sqlite {
    pool: Quaint,
    file_path: String,
}

impl Sqlite {
    pub fn file_path(&self) -> &str {
        self.file_path.as_str()
    }
}

impl FromSource for Sqlite {
    fn from_source(source: &dyn Source) -> crate::Result<Self> {
        let params = SqliteParams::try_from(source.url().value.as_str())?;
        let db_name = std::path::Path::new(&params.file_path).file_stem().unwrap().to_str().unwrap().to_owned();
        let file_path = params.file_path;

        let url_with_db = {
            let mut url = Url::parse(&source.url().value)?;
            url.query_pairs_mut().append_pair("db_name", &db_name);
            url
        };

        let pool = Quaint::new(url_with_db.as_str())?;

        Ok(Self { pool, file_path })
    }
}

impl Connector for Sqlite {
    fn get_connection<'a>(&'a self) -> IO<Box<dyn Connection + 'a>> {
        IO::new(async move {
            let conn = self.pool.check_out().await.map_err(SqlError::from)?;
            let conn = SqlConnection::<_, ManyRelatedRecordsWithRowNumber>::new(conn);

            Ok(Box::new(conn) as Box<dyn Connection>)
        })
    }
}
