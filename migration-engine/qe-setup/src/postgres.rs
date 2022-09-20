use migration_core::migration_connector::{ConnectorError, ConnectorResult};
use quaint::{prelude::*, single::Quaint};
use std::collections::HashMap;
use url::Url;

pub(crate) async fn postgres_setup(url: String, prisma_schema: &str) -> ConnectorResult<()> {
    let mut url = Url::parse(&url).map_err(ConnectorError::url_parse_error)?;
    let quaint_url = quaint::connector::PostgresUrl::new(url.clone()).unwrap();
    let db_name = quaint_url.dbname();
    strip_schema_param_from_url(&mut url);
    let conn = create_postgres_admin_conn(url.clone()).await?;

    let query = format!("DROP DATABASE IF EXISTS \"{}\"", db_name);
    conn.raw_cmd(&query).await.unwrap();

    let query = format!("CREATE DATABASE \"{}\"", db_name);
    conn.raw_cmd(&query).await.unwrap();

    crate::diff_and_apply(prisma_schema).await;
    Ok(())
}

async fn create_postgres_admin_conn(mut url: Url) -> ConnectorResult<Quaint> {
    url.set_path("/postgres");
    Ok(Quaint::new(url.as_ref()).await.unwrap())
}

fn strip_schema_param_from_url(url: &mut Url) {
    let mut params: HashMap<String, String> = url.query_pairs().into_owned().collect();
    params.remove("schema");
    let params: Vec<String> = params.into_iter().map(|(k, v)| format!("{}={}", k, v)).collect();
    let params: String = params.join("&");
    url.set_query(Some(&params));
}
