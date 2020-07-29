mod types;

use super::type_test::TypeTest;
use crate::{connector::Queryable, single::Quaint};
use names::Generator;
use once_cell::sync::Lazy;
use std::env;

static CONN_STR: Lazy<String> = Lazy::new(|| env::var("TEST_MSSQL").expect("TEST_MSSQL env var"));

pub struct MsSql<'a> {
    names: Generator<'a>,
    conn: Quaint,
}

#[async_trait::async_trait]
impl<'a> TypeTest for MsSql<'a> {
    async fn new() -> crate::Result<MsSql<'a>> {
        let names = Generator::default();
        let conn = Quaint::new(&CONN_STR).await?;

        Ok(Self { names, conn })
    }

    async fn create_table(&mut self, r#type: &str) -> crate::Result<String> {
        let table = format!("##{}", self.names.next().unwrap().replace('-', ""));

        let create_table = format!(
            r##"
            CREATE TABLE {} (
                id INT IDENTITY(1,1) PRIMARY KEY,
                value {}
            )
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
