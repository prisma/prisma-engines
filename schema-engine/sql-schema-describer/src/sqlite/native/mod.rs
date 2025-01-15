use crate::{sqlite::Connection, DescriberResult, SqlMetadata, SqlSchema, SqlSchemaDescriberBackend};

use quaint::{
    connector::{rusqlite, ColumnType as QuaintColumnType, GetRow, ToColumnNames},
    prelude::{ResultSet, Value},
};

use super::SqlSchemaDescriber;

#[async_trait::async_trait]
impl Connection for std::sync::Mutex<rusqlite::Connection> {
    async fn query_raw<'a>(&'a self, sql: &'a str, params: &'a [Value<'a>]) -> quaint::Result<ResultSet> {
        let conn = self.lock().unwrap();
        let mut stmt = conn.prepare_cached(sql)?;
        let column_types = stmt.columns().iter().map(QuaintColumnType::from).collect::<Vec<_>>();
        let mut rows = stmt.query(rusqlite::params_from_iter(params.iter()))?;
        let column_names = rows.to_column_names();
        let mut converted_rows = Vec::new();
        while let Some(row) = rows.next()? {
            converted_rows.push(row.get_result_row().unwrap());
        }

        Ok(ResultSet::new(column_names, column_types, converted_rows))
    }
}

#[async_trait::async_trait]
impl SqlSchemaDescriberBackend for SqlSchemaDescriber<'_> {
    async fn list_databases(&self) -> DescriberResult<Vec<String>> {
        Ok(self.get_databases().await?)
    }

    async fn get_metadata(&self, _schema: &str) -> DescriberResult<SqlMetadata> {
        let mut sql_schema = SqlSchema::default();
        let table_count = self.get_table_names(&mut sql_schema).await?.len();
        let size_in_bytes = self.get_size().await?;

        Ok(SqlMetadata {
            table_count,
            size_in_bytes,
        })
    }

    async fn describe(&self, _schemas: &[&str]) -> DescriberResult<SqlSchema> {
        self.describe_impl().await
    }

    async fn version(&self) -> DescriberResult<Option<String>> {
        Ok(Some(quaint::connector::sqlite_version().to_owned()))
    }
}
