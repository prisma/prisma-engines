mod conversion;
mod error;

use mysql as my;
use url::Url;

use crate::{
    ast::{Id, ParameterizedValue, Query},
    connector::{queryable::*, ResultSet, Transaction},
    visitor::{self, Visitor},
};

/// A connector interface for the MySQL database.
pub struct Mysql {
    pub(crate) client: my::Conn,
}

impl From<my::Conn> for Mysql {
    fn from(client: my::Conn) -> Self {
        Self { client }
    }
}

impl Mysql {
    pub fn new(conf: mysql::OptsBuilder) -> crate::Result<Self> {
        let client = my::Conn::new(conf)?;
        Ok(Mysql { client })
    }

    pub fn new_from_url(url: &str) -> crate::Result<Self> {
        let mut builder = my::OptsBuilder::new();
        let url = Url::parse(url)?;
        let db_name = url.path_segments().and_then(|mut segments| segments.next());

        builder.ip_or_hostname(url.host_str());
        builder.tcp_port(url.port().unwrap_or(3306));
        builder.user(Some(url.username()));
        builder.pass(url.password());
        builder.db_name(db_name);
        builder.verify_peer(false);
        builder.stmt_cache_size(Some(1000));

        let client = my::Conn::new(builder)?;

        Ok(Mysql { client })
    }
}

impl Queryable for Mysql {
    fn execute(&mut self, q: Query) -> crate::Result<Option<Id>> {
        let (sql, params) = dbg!(visitor::Mysql::build(q));

        let mut stmt = self.client.prepare(&sql)?;
        let result = stmt.execute(params)?;

        Ok(Some(Id::from(result.last_insert_id())))
    }

    fn query<'a>(&mut self, q: Query<'a>) -> crate::Result<ResultSet> {
        let (sql, params) = dbg!(visitor::Mysql::build(q));
        self.query_raw(&sql, &params[..])
    }

    fn query_raw<'a>(
        &mut self,
        sql: &str,
        params: &[ParameterizedValue<'a>],
    ) -> crate::Result<ResultSet> {
        let mut stmt = self.client.prepare(&sql)?;
        let mut result = ResultSet::new(stmt.to_column_names(), Vec::new());
        let rows = stmt.execute(conversion::conv_params(params))?;

        for row in rows {
            result.rows.push(row?.to_result_row()?);
        }

        Ok(result)
    }

    fn execute_raw<'a>(
        &mut self,
        sql: &str,
        params: &[ParameterizedValue<'a>],
    ) -> crate::Result<u64> {
        let mut stmt = self.client.prepare(sql)?;
        let result = stmt.execute(conversion::conv_params(params))?;

        Ok(result.affected_rows())
    }

    fn turn_off_fk_constraints(&mut self) -> crate::Result<()> {
        self.client.query("SET FOREIGN_KEY_CHECKS=0")?;
        Ok(())
    }

    fn turn_on_fk_constraints(&mut self) -> crate::Result<()> {
        self.client.query("SET FOREIGN_KEY_CHECKS=1")?;
        Ok(())
    }

    fn start_transaction<'b>(&'b mut self) -> crate::Result<Transaction<'b>> {
        Ok(Transaction::new(self)?)
    }

    fn raw_cmd(&mut self, cmd: &str) -> crate::Result<()> {
        self.client.query(cmd)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::connector::Queryable;
    use mysql::OptsBuilder;
    use std::env;

    fn get_config() -> OptsBuilder {
        let mut config = OptsBuilder::new();
        config.ip_or_hostname(env::var("TEST_MYSQL_HOST").ok());
        config.tcp_port(env::var("TEST_MYSQL_PORT").unwrap().parse::<u16>().unwrap());
        config.db_name(env::var("TEST_MYSQL_DB").ok());
        config.pass(env::var("TEST_MYSQL_PASSWORD").ok());
        config.user(env::var("TEST_MYSQL_USER").ok());
        config
    }

    #[test]
    fn should_provide_a_database_connection() {
        let mut connection = Mysql::new(get_config()).unwrap();

        let res = connection
            .query_raw(
                "select * from information_schema.`COLUMNS` where COLUMN_NAME = 'unknown_123'",
                &[],
            )
            .unwrap();

        assert!(res.is_empty());
    }

    #[test]
    fn should_provide_a_database_transaction() {
        let mut connection = Mysql::new(get_config()).unwrap();
        let mut tx = connection.start_transaction().unwrap();

        let res = tx
            .query_raw(
                "select * from information_schema.`COLUMNS` where COLUMN_NAME = 'unknown_123'",
                &[],
            )
            .unwrap();

        assert!(res.is_empty());
    }

    const TABLE_DEF: &str = r#"
CREATE TABLE `user`(
    id       int4    PRIMARY KEY     NOT NULL,
    name     text    NOT NULL,
    age      int4    NOT NULL,
    salary   float4
);
"#;

    const CREATE_USER: &str = r#"
INSERT INTO `user` (id, name, age, salary)
VALUES (1, 'Joe', 27, 20000.00 );
"#;

    const DROP_TABLE: &str = "DROP TABLE IF EXISTS `user`;";

    #[test]
    fn should_map_columns_correctly() {
        let mut connection = Mysql::new(get_config()).unwrap();

        connection.query_raw(DROP_TABLE, &[]).unwrap();
        connection.query_raw(TABLE_DEF, &[]).unwrap();
        connection.query_raw(CREATE_USER, &[]).unwrap();

        let rows = connection.query_raw("SELECT * FROM `user`", &[]).unwrap();
        assert_eq!(rows.len(), 1);

        let row = rows.get(0).unwrap();
        assert_eq!(row["id"].as_i64(), Some(1));
        assert_eq!(row["name"].as_str(), Some("Joe"));
        assert_eq!(row["age"].as_i64(), Some(27));
        assert_eq!(row["salary"].as_f64(), Some(20000.0));
    }
}
