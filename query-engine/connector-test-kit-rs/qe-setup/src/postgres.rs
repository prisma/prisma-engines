use quaint::{connector::PostgresFlavour, prelude::*, single::Quaint};
use schema_core::schema_connector::{ConnectorError, ConnectorParams, ConnectorResult};
use std::collections::HashMap;
use url::Url;

pub(crate) async fn postgres_setup(url: String, prisma_schema: &str, db_schemas: &[&str]) -> ConnectorResult<()> {
    let mut parsed_url = Url::parse(&url).map_err(ConnectorError::url_parse_error)?;
    let mut quaint_url = quaint::connector::PostgresNativeUrl::new(parsed_url.clone()).unwrap();
    quaint_url.set_flavour(PostgresFlavour::Postgres);

    let (db_name, schema) = (quaint_url.dbname(), quaint_url.schema());

    if !db_schemas.is_empty() {
        strip_schema_param_from_url(&mut parsed_url);
        let conn = create_postgres_admin_conn(parsed_url.clone()).await?;

        let query = format!("DROP DATABASE \"{db_name}\"");
        conn.raw_cmd(&query).await.ok();

        let query = format!("CREATE DATABASE \"{db_name}\"");
        conn.raw_cmd(&query).await.ok();
    } else {
        strip_schema_param_from_url(&mut parsed_url);
        let conn = create_postgres_admin_conn(parsed_url.clone()).await?;

        let query = format!("CREATE DATABASE \"{db_name}\"");
        conn.raw_cmd(&query).await.ok();

        // Now create the schema
        parsed_url.set_path(&format!("/{db_name}"));

        let conn = Quaint::new(parsed_url.as_ref()).await.unwrap();

        let drop_and_recreate_schema =
            format!("DROP SCHEMA IF EXISTS \"{schema}\" CASCADE;\nCREATE SCHEMA \"{schema}\";");
        conn.raw_cmd(&drop_and_recreate_schema)
            .await
            .map_err(|e| ConnectorError::from_source(e, ""))?;
    }

    let params = ConnectorParams::new(url, Default::default(), None);
    let mut connector = sql_schema_connector::SqlSchemaConnector::new_postgres(params)?;
    crate::diff_and_apply(prisma_schema, &mut connector).await
}

pub(crate) async fn postgres_teardown(url: &str, db_schemas: &[&str]) -> ConnectorResult<()> {
    // only teardown if we doing multischema
    if !db_schemas.is_empty() {
        let mut url = Url::parse(url).map_err(ConnectorError::url_parse_error)?;
        strip_schema_param_from_url(&mut url);

        let conn = create_postgres_admin_conn(url.clone()).await?;
        let db_name = url.path().strip_prefix('/').unwrap();

        let query = format!("DROP DATABASE \"{db_name}\" CASCADE");
        conn.raw_cmd(&query).await.ok();
    }

    Ok(())
}

async fn create_postgres_admin_conn(mut url: Url) -> ConnectorResult<Quaint> {
    url.set_path("/postgres");
    Ok(Quaint::new(url.as_ref()).await.unwrap())
}

fn strip_schema_param_from_url(url: &mut Url) {
    let mut params: HashMap<String, String> = url.query_pairs().into_owned().collect();
    params.remove("schema");
    let params: Vec<String> = params.into_iter().map(|(k, v)| format!("{k}={v}")).collect();
    let params: String = params.join("&");
    url.set_query(Some(&params));
}
