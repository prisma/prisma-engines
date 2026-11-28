use super::TestApi;
use crate::{connector::Queryable, single::Quaint};
use names::Generator;
use quaint_test_setup::Tags;
use std::{env, sync::LazyLock};

pub static CONN_STR: LazyLock<String> =
    LazyLock::new(|| env::var("TEST_KINGBASE_MYSQL").expect("TEST_KINGBASE_MYSQL env var"));

pub(crate) async fn kingbase_mysql_test_api<'a>() -> crate::Result<KingbaseMysql<'a>> {
    KingbaseMysql::new(CONN_STR.as_str()).await
}

pub struct KingbaseMysql<'a> {
    names: Generator<'a>,
    conn: Quaint,
    conn_str: String,
}

impl<'a> KingbaseMysql<'a> {
    pub async fn new(conn_str: &str) -> crate::Result<KingbaseMysql<'a>> {
        let names = Generator::default();
        let conn = Quaint::new(conn_str).await?;

        Ok(Self {
            names,
            conn,
            conn_str: conn_str.to_owned(),
        })
    }
}

#[async_trait::async_trait]
impl TestApi for KingbaseMysql<'_> {
    fn system(&self) -> &'static str {
        // Reuse the MySQL query-test branches for MySQL-compatible syntax.
        "mysql"
    }

    async fn create_type_table(&mut self, r#type: &str) -> crate::Result<String> {
        self.create_temp_table(&format!("{}, `value` {}", self.autogen_id("id"), r#type))
            .await
    }

    async fn create_temp_table(&mut self, columns: &str) -> crate::Result<String> {
        let name = self.get_name();
        let (name, create) = self.render_create_table(&name, columns);

        self.conn().raw_cmd(&create).await?;

        Ok(name)
    }

    async fn create_table(&mut self, columns: &str) -> crate::Result<String> {
        let name = self.get_name();
        let create = format!("CREATE TABLE `{name}` ({columns})");

        self.conn().raw_cmd(&create).await?;

        Ok(name)
    }

    async fn delete_table(&mut self, table_name: &str) -> crate::Result<()> {
        self.conn().raw_cmd(&format!("DROP TABLE `{table_name}`")).await
    }

    fn render_create_table(&mut self, table_name: &str, columns: &str) -> (String, String) {
        let create = format!("CREATE TEMPORARY TABLE `{table_name}` ({columns})");

        (table_name.to_owned(), create)
    }

    async fn create_index(&mut self, table: &str, columns: &str) -> crate::Result<String> {
        let name = self.get_name();
        let create = format!("CREATE UNIQUE INDEX {name} ON {table} ({columns})");

        self.conn().raw_cmd(&create).await?;

        Ok(name)
    }

    fn conn(&self) -> &Quaint {
        &self.conn
    }

    async fn create_additional_connection(&self) -> crate::Result<Quaint> {
        Quaint::new(&self.conn_str).await
    }

    fn create_pool(&self) -> crate::Result<crate::pooled::Quaint> {
        Ok(crate::pooled::Quaint::builder(&CONN_STR)?.build())
    }

    fn unique_constraint(&mut self, column: &str) -> String {
        format!("UNIQUE({column})")
    }

    fn foreign_key(&mut self, parent_table: &str, parent_column: &str, child_column: &str) -> String {
        let name = self.get_name();

        format!("CONSTRAINT {name} FOREIGN KEY ({child_column}) REFERENCES {parent_table}({parent_column})")
    }

    fn autogen_id(&self, name: &str) -> String {
        format!("{name} INT AUTO_INCREMENT PRIMARY KEY")
    }

    fn get_name(&mut self) -> String {
        self.names.next().unwrap().replace('-', "")
    }

    fn connector_tag(&self) -> Tags {
        Tags::KINGBASE_MYSQL
    }
}
