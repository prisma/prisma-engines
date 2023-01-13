use migration_core::migration_connector::{ConnectorError, ConnectorResult};
use quaint::{prelude::*, single::Quaint};
use url::Url;

pub(crate) async fn cockroach_setup(url: String, prisma_schema: &str) -> ConnectorResult<()> {
    let url = Url::parse(&url).map_err(ConnectorError::url_parse_error)?;
    let quaint_url = quaint::connector::PostgresUrl::new(url.clone()).unwrap();
    let db_name = quaint_url.dbname();
    let conn = create_admin_conn(url).await?;

    let query = format!(
        r#"
        DROP DATABASE IF EXISTS "{db_name}";
        CREATE DATABASE "{db_name}";
        "#
    );

    conn.raw_cmd(&query).await.unwrap();
    crate::diff_and_apply(prisma_schema).await;

    Ok(())
}

async fn create_admin_conn(mut url: Url) -> ConnectorResult<Quaint> {
    url.set_path("/postgres");
    Ok(Quaint::new(url.as_ref()).await.unwrap())
}
