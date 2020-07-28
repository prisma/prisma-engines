mod types;

use super::type_test::TypeTest;
use crate::{connector::Queryable, single::Quaint};
use names::Generator;
use once_cell::sync::Lazy;
use std::env;

static CONN_STR: Lazy<String> = Lazy::new(|| env::var("TEST_MYSQL").expect("TEST_MYSQL env var"));

pub struct MySql<'a> {
    names: Generator<'a>,
    conn: Quaint,
}

#[async_trait::async_trait]
impl<'a> TypeTest for MySql<'a> {
    async fn new() -> crate::Result<MySql<'a>> {
        let names = Generator::default();
        let conn = Quaint::new(&CONN_STR).await?;

        Ok(Self { names, conn })
    }

    async fn create_table(&mut self, r#type: &str) -> crate::Result<String> {
        let table = self.names.next().unwrap().replace('-', "");

        let create_table = format!(
            r##"
            CREATE TEMPORARY TABLE `{}` (
                `id` int(11) NOT NULL AUTO_INCREMENT,
                `value` {},
                PRIMARY KEY (`id`)
            ) ENGINE=InnoDB DEFAULT CHARSET=latin1
            "##,
            table, r#type,
        );

        self.conn.raw_cmd(&create_table).await?;

        Ok(table)
    }

    fn conn(&self) -> &Quaint {
        &self.conn
    }
}
