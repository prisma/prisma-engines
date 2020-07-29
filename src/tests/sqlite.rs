mod types;

use super::type_test::TypeTest;
use crate::{connector::Queryable, single::Quaint};
use names::Generator;

pub struct Sqlite<'a> {
    names: Generator<'a>,
    conn: Quaint,
}

#[async_trait::async_trait]
impl<'a> TypeTest for Sqlite<'a> {
    async fn new() -> crate::Result<Sqlite<'a>> {
        let names = Generator::default();
        let conn = Quaint::new("file:db/test.db").await?;

        Ok(Self { names, conn })
    }

    async fn create_table(&mut self, r#type: &str) -> crate::Result<String> {
        let table = self.names.next().unwrap().replace('-', "");

        let create_table = format!(
            r##"
            CREATE TEMPORARY TABLE `{}` (
                `id` SERIAL PRIMARY KEY,
                `value` {}
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
