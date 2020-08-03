pub mod mssql;
pub mod mysql;
pub mod postgres;
pub mod sqlite;

pub use mssql::*;
pub use mysql::*;
pub use postgres::*;
pub use sqlite::*;

#[async_trait::async_trait]
pub trait Connector {
    async fn new() -> crate::Result<Self>
    where
        Self: Sized;

    async fn create_type_table(&mut self, r#type: &str) -> crate::Result<String>;
    async fn create_table(&mut self, columns: &str) -> crate::Result<String>;

    async fn create_index(&mut self, table: &str, columns: &str) -> crate::Result<String>;

    fn system(&self) -> &'static str;
    fn unique_constraint(&mut self, column: &str) -> String;
    fn foreign_key(&mut self, parent_table: &str, parent_column: &str, child_column: &str) -> String;
    fn autogen_id(&self, name: &str) -> String;
    fn conn(&self) -> &crate::single::Quaint;
    fn get_name(&mut self) -> String;
}
