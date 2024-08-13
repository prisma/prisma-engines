use quaint_test_setup::Tags;

#[cfg(feature = "mssql")]
pub mod mssql;
#[cfg(feature = "mysql")]
pub mod mysql;
#[cfg(feature = "postgresql")]
pub mod postgres;
#[cfg(feature = "sqlite")]
pub mod sqlite;

#[cfg(feature = "mssql")]
pub use mssql::*;
#[cfg(feature = "mysql")]
pub use mysql::*;
#[cfg(feature = "postgresql")]
pub use postgres::*;
#[cfg(feature = "sqlite")]
pub use sqlite::*;

#[async_trait::async_trait]
pub trait TestApi {
    async fn create_type_table(&mut self, r#type: &str) -> crate::Result<String>;
    fn render_create_table(&mut self, table_name: &str, columns: &str) -> (String, String);
    async fn create_temp_table(&mut self, columns: &str) -> crate::Result<String>;
    async fn create_table(&mut self, columns: &str) -> crate::Result<String>;
    async fn delete_table(&mut self, table_name: &str) -> crate::Result<()>;

    async fn create_index(&mut self, table: &str, columns: &str) -> crate::Result<String>;

    fn system(&self) -> &'static str;
    fn connector_tag(&self) -> Tags;
    fn unique_constraint(&mut self, column: &str) -> String;
    fn foreign_key(&mut self, parent_table: &str, parent_column: &str, child_column: &str) -> String;
    fn autogen_id(&self, name: &str) -> String;
    fn conn(&self) -> &crate::single::Quaint;
    async fn create_additional_connection(&self) -> crate::Result<crate::single::Quaint>;
    fn get_name(&mut self) -> String;
}
