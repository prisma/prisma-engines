use quaint::{connector::PostgresFlavour, prelude::*, single::Quaint};
use schema_core::schema_connector::{ConnectorError, ConnectorResult};
use url::Url;

pub(crate) async fn cockroach_setup(url: String, prisma_schema: &str) -> ConnectorResult<()> {
    let mut parsed_url = Url::parse(&url).map_err(ConnectorError::url_parse_error)?;
    let mut quaint_url = quaint::connector::PostgresUrl::new(parsed_url.clone()).unwrap();
    quaint_url.set_flavour(PostgresFlavour::Cockroach);

    let db_name = quaint_url.dbname();
    let conn = create_admin_conn(&mut parsed_url).await?;

    let query = format!(
        r#"
        DROP DATABASE IF EXISTS "{db_name}";
        CREATE DATABASE "{db_name}";
        "#
    );

    conn.raw_cmd(&query).await.unwrap();

    let mut connector = sql_schema_connector::SqlSchemaConnector::new_cockroach();
    crate::diff_and_apply(prisma_schema, url, &mut connector).await
}

async fn create_admin_conn(url: &mut Url) -> ConnectorResult<Quaint> {
    url.set_path("/postgres");
    Ok(Quaint::new(url.as_ref()).await.unwrap())
}
