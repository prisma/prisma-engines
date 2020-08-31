use super::TestApi;
use crate::{connector::Queryable, single::Quaint};
use names::Generator;

pub(crate) async fn sqlite_test_api<'a>() -> crate::Result<Sqlite<'a>> {
    Sqlite::new().await
}

pub struct Sqlite<'a> {
    names: Generator<'a>,
    conn: Quaint,
}

#[async_trait::async_trait]
impl<'a> TestApi for Sqlite<'a> {
    async fn new() -> crate::Result<Self> {
        let names = Generator::default();
        let conn_str = "file:db/test.db";
        let conn = Quaint::new(&conn_str).await?;

        Ok(Self { names, conn })
    }

    fn system(&self) -> &'static str {
        "sqlite"
    }

    async fn create_type_table(&mut self, r#type: &str) -> crate::Result<String> {
        self.create_table(&format!("{}, `value` {}", self.autogen_id("id"), r#type))
            .await
    }

    async fn create_table(&mut self, columns: &str) -> crate::Result<String> {
        let name = self.get_name();

        let create = format!(
            r##"
            CREATE TEMPORARY TABLE `{}` ({})
            "##,
            name, columns,
        );

        self.conn().raw_cmd(&create).await?;

        Ok(name)
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
        format!(
            "FOREIGN KEY ({}) REFERENCES {}({})",
            child_column, parent_table, parent_column
        )
    }

    fn autogen_id(&self, name: &str) -> String {
        format!("{} INTEGER PRIMARY KEY", name)
    }

    fn get_name(&mut self) -> String {
        self.names.next().unwrap().replace('-', "")
    }
}
