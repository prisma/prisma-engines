use crate::sqlite::Connection;

use quaint::{
    connector::{rusqlite, ColumnType as QuaintColumnType, GetRow, ToColumnNames},
    prelude::{ResultSet, Value},
};

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
