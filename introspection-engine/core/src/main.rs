mod error;

use error::*;
use prisma_query::ast::*;
use prisma_query::connector::{PostgreSql, Queryable, Sqlite, SqliteParams};
use sql_schema_describer::SqlSchemaDescriberBackend;
use std::convert::TryFrom;
use std::io;
use std::sync::{Arc, Mutex};
use url::Url;

fn main() {
    let mut input = String::new();
    io::stdin()
        .read_line(&mut input)
        .expect("Reading datasource url from stdin failed");

    let data_source_url = input.trim_end_matches('\n'); // read_line appends a line break

    doit(&data_source_url).expect("Introspection Failed");
}

fn doit(url: &str) -> CoreResult<()> {
    let database_schema = load_connector(&url)?.describe("")?;
    let data_model = introspection_command::calculate_model(&database_schema).unwrap();
    Ok(datamodel::render_to(&mut std::io::stdout().lock(), &data_model).unwrap())
}

fn load_connector(url_str: &str) -> CoreResult<Box<dyn SqlSchemaDescriberBackend>> {
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
        Err(CoreError::InvalidUrl {
            message: format!("Could not load connector for the provided url: {}", url_str),
        })
    }
}

pub struct PostgresWrapper {
    conn: Mutex<PostgreSql>,
}

impl PostgresWrapper {
    fn new(url_str: &str) -> CoreResult<Self> {
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

pub struct SqliteWrapper {
    conn: Mutex<Sqlite>,
    file_path: String,
}

impl SqliteWrapper {
    fn new(url: &str) -> CoreResult<Self> {
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
