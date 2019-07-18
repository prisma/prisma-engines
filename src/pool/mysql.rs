use super::PrismaConnectionManager;
use crate::{
    connector::{Mysql, Queryable},
    error::Error,
};
use failure::{Compat, Fail};
use r2d2::ManageConnection;

pub use mysql::OptsBuilder;
pub use r2d2_mysql::MysqlConnectionManager;

impl From<OptsBuilder> for PrismaConnectionManager<MysqlConnectionManager> {
    fn from(opts: OptsBuilder) -> Self {
        Self {
            inner: MysqlConnectionManager::new(opts),
            file_path: None,
        }
    }
}

impl ManageConnection for PrismaConnectionManager<MysqlConnectionManager> {
    type Connection = Mysql;
    type Error = Compat<Error>;

    fn connect(&self) -> Result<Self::Connection, Self::Error> {
        match self.inner.connect() {
            Ok(client) => Ok(Mysql::from(client)),
            Err(e) => Err(Error::from(e).compat()),
        }
    }

    fn is_valid(&self, conn: &mut Self::Connection) -> Result<(), Self::Error> {
        match conn.query_raw("", &[]) {
            Ok(_) => Ok(()),
            Err(e) => Err(Error::from(e).compat()),
        }
    }

    fn has_broken(&self, _: &mut Self::Connection) -> bool {
        false
    }
}
