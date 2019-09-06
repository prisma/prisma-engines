use crate::{SqlIntrospectionError, SqlIntrospectionResult};
use prisma_query::ast::*;
use prisma_query::connector::{PostgreSql, Queryable, Sqlite, SqliteParams};
use sql_schema_describer::SqlSchemaDescriberBackend;
use std::convert::TryFrom;
use std::sync::{Arc, Mutex};
use url::Url;

pub fn load_describer(url_str: &str) -> SqlIntrospectionResult<Box<dyn SqlSchemaDescriberBackend>> {
    if url_str.starts_with("postgresql://") {
        let wrapper = PostgresWrapper::new(&url_str)?;
        Ok(Box::new(sql_schema_describer::postgres::SqlSchemaDescriber::new(
            Arc::new(wrapper),
        )))
    } else if url_str.starts_with("file:") {
        let wrapper = SqliteWrapper::new(url_str)?;
        Ok(Box::new(sql_schema_describer::sqlite::SqlSchemaDescriber::new(
            Arc::new(wrapper),
        )))
    } else {
        Err(SqlIntrospectionError::InvalidUrl {
            message: format!("Could not load connector for the provided url: {}", url_str),
        })
    }
}

struct PostgresWrapper {
    conn: Mutex<PostgreSql>,
}

impl PostgresWrapper {
    fn new(url_str: &str) -> SqlIntrospectionResult<Self> {
        let url = Url::parse(url_str)?;
        let queryable = PostgreSql::try_from(url.clone())?;
        Ok(PostgresWrapper {
            conn: Mutex::new(queryable),
        })
    }
}

impl sql_schema_describer::SqlConnection for PostgresWrapper {
    fn query_raw(
        &self,
        sql: &str,
        _schema: &str,
        params: &[ParameterizedValue],
    ) -> prisma_query::Result<prisma_query::connector::ResultSet> {
        self.conn.lock().unwrap().query_raw(sql, params)
    }
}

struct SqliteWrapper {
    conn: Mutex<Sqlite>,
    file_path: String,
}

impl SqliteWrapper {
    fn new(url: &str) -> SqlIntrospectionResult<Self> {
        let params = SqliteParams::try_from(url)?;
        let queryable = Sqlite::try_from(url)?;
        Ok(SqliteWrapper {
            conn: Mutex::new(queryable),
            file_path: params.file_path.to_str().unwrap().to_string(),
        })
    }
}

impl sql_schema_describer::SqlConnection for SqliteWrapper {
    fn query_raw(
        &self,
        sql: &str,
        schema: &str,
        params: &[ParameterizedValue],
    ) -> prisma_query::Result<prisma_query::connector::ResultSet> {
        let mut conn = self.conn.lock().unwrap();

        conn.execute_raw(
            "ATTACH DATABASE ? AS ?",
            &[
                ParameterizedValue::from(self.file_path.as_str()),
                ParameterizedValue::from(schema),
            ],
        )
        .unwrap();

        let res = conn.query_raw(sql, params);

        conn.execute_raw("DETACH DATABASE ?", &[ParameterizedValue::from(schema)])
            .unwrap();

        res
    }
}
