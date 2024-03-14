use super::TestApi;
use crate::{connector::Queryable, single::Quaint};
use names::Generator;
use once_cell::sync::Lazy;
use quaint_test_setup::Tags;
use std::env;

pub static CONN_STR: Lazy<String> = Lazy::new(|| env::var("TEST_PSQL").expect("TEST_PSQL env var"));
pub static CRDB_CONN_STR: Lazy<String> = Lazy::new(|| env::var("TEST_CRDB").expect("TEST_CRDB env var"));

pub(crate) async fn postgresql_test_api<'a>() -> crate::Result<PostgreSql<'a>> {
    PostgreSql::new().await
}

pub struct PostgreSql<'a> {
    names: Generator<'a>,
    conn: Quaint,
}

impl<'a> PostgreSql<'a> {
    pub async fn new() -> crate::Result<PostgreSql<'a>> {
        let names = Generator::default();
        let conn = Quaint::new(&CONN_STR).await?;

        Ok(Self { names, conn })
    }
}

#[async_trait::async_trait]
impl<'a> TestApi for PostgreSql<'a> {
    fn system(&self) -> &'static str {
        "postgres"
    }

    async fn create_type_table(&mut self, r#type: &str) -> crate::Result<String> {
        self.create_temp_table(&format!("{}, value {}", self.autogen_id("id"), r#type))
            .await
    }

    async fn create_table(&mut self, _columns: &str) -> crate::Result<String> {
        unimplemented!("only required for MySql nested sub select test")
    }

    async fn delete_table(&mut self, _table_name: &str) -> crate::Result<()> {
        unimplemented!("only required for MySql nested sub select test")
    }

    async fn create_temp_table(&mut self, columns: &str) -> crate::Result<String> {
        let name = self.get_name();

        let (name, create) = self.render_create_table(&name, columns);

        self.conn().raw_cmd(&create).await?;

        Ok(name)
    }

    fn render_create_table(&mut self, table_name: &str, columns: &str) -> (String, String) {
        let create = format!(
            r##"
            CREATE TEMPORARY TABLE "{table_name}" ({columns})
            "##,
        );

        (table_name.to_string(), create)
    }

    async fn create_index(&mut self, table: &str, columns: &str) -> crate::Result<String> {
        let name = self.get_name();

        let create = format!(
            r##"
            CREATE UNIQUE INDEX {name} ON {table} ({columns})
            "##
        );

        self.conn().raw_cmd(&create).await?;

        Ok(name)
    }

    fn conn(&self) -> &Quaint {
        &self.conn
    }

    async fn create_additional_connection(&self) -> crate::Result<Quaint> {
        Quaint::new(&CONN_STR).await
    }

    fn unique_constraint(&mut self, column: &str) -> String {
        format!("UNIQUE({column})")
    }

    fn foreign_key(&mut self, parent_table: &str, parent_column: &str, child_column: &str) -> String {
        let name = self.get_name();

        format!(
            "CONSTRAINT {} FOREIGN KEY ({}) REFERENCES {}({})",
            &name, child_column, parent_table, parent_column
        )
    }

    fn autogen_id(&self, name: &str) -> String {
        format!("{name} SERIAL PRIMARY KEY")
    }

    fn get_name(&mut self) -> String {
        self.names.next().unwrap().replace('-', "")
    }

    fn connector_tag(&self) -> quaint_test_setup::Tags {
        Tags::POSTGRES
    }
}
