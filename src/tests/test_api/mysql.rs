use super::TestApi;
use crate::{connector::Queryable, single::Quaint};
use names::Generator;
use once_cell::sync::Lazy;
use std::env;

pub static CONN_STR: Lazy<String> = Lazy::new(|| env::var("TEST_MYSQL").expect("TEST_MYSQL env var"));
pub static CONN_STR8: Lazy<String> = Lazy::new(|| env::var("TEST_MYSQL8").expect("TEST_MYSQL8 env var"));

pub(crate) async fn mysql_test_api<'a>() -> crate::Result<MySql<'a>> {
    MySql::new(CONN_STR.as_str()).await
}

pub(crate) async fn mysql8_test_api<'a>() -> crate::Result<MySql<'a>> {
    MySql::new(CONN_STR8.as_str()).await
}

pub struct MySql<'a> {
    names: Generator<'a>,
    conn: Quaint,
}

impl<'a> MySql<'a> {
    pub async fn new(conn_str: &str) -> crate::Result<MySql<'a>> {
        let names = Generator::default();
        let conn = Quaint::new(conn_str).await?;

        Ok(Self { names, conn })
    }
}

#[async_trait::async_trait]
impl<'a> TestApi for MySql<'a> {
    fn system(&self) -> &'static str {
        "mysql"
    }

    async fn create_type_table(&mut self, r#type: &str) -> crate::Result<String> {
        self.create_table(&format!("{}, `value` {}", self.autogen_id("id"), r#type))
            .await
    }

    async fn create_table(&mut self, columns: &str) -> crate::Result<String> {
        let name = self.get_name();

        let (name, create) = self.render_create_table(&name, columns);

        self.conn().raw_cmd(&create).await?;

        Ok(name)
    }

    fn render_create_table(&mut self, table_name: &str, columns: &str) -> (String, String) {
        let create = format!(
            r##"
            CREATE TEMPORARY TABLE `{}` ({}) ENGINE=InnoDB DEFAULT CHARSET=latin1
            "##,
            table_name, columns,
        );

        (table_name.to_string(), create)
    }

    async fn create_index(&mut self, table: &str, columns: &str) -> crate::Result<String> {
        let name = self.get_name();

        let create = format!(
            r##"
            CREATE UNIQUE INDEX {} ON {} ({})
            "##,
            name, table, columns
        );

        self.conn().raw_cmd(&create).await?;

        Ok(name)
    }

    fn conn(&self) -> &Quaint {
        &self.conn
    }

    fn unique_constraint(&mut self, column: &str) -> String {
        format!("UNIQUE({})", column)
    }

    fn foreign_key(&mut self, parent_table: &str, parent_column: &str, child_column: &str) -> String {
        let name = self.get_name();

        format!(
            "CONSTRAINT {} FOREIGN KEY ({}) REFERENCES {}({})",
            &name, child_column, parent_table, parent_column
        )
    }

    fn autogen_id(&self, name: &str) -> String {
        format!("{} INT(11) NOT NULL AUTO_INCREMENT PRIMARY KEY", name)
    }

    fn get_name(&mut self) -> String {
        self.names.next().unwrap().replace('-', "")
    }
}
