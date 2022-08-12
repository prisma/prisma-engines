use migration_core::migration_connector::{ConnectorError, ConnectorResult};
use quaint::{prelude::*, single::Quaint};
use url::Url;

pub(crate) async fn mysql_reset(url: &str) -> ConnectorResult<()> {
    let mut url = Url::parse(url).map_err(ConnectorError::url_parse_error)?;
    let db_name = url.path().trim_start_matches('/').to_owned();
    url.set_path("/mysql");

    let conn = Quaint::new(url.as_ref()).await.unwrap();

    let query = format!("DROP DATABASE IF EXISTS `{}`", db_name);
    conn.raw_cmd(&query).await.unwrap();

    let query = format!(
        "CREATE DATABASE `{}` CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci;",
        db_name
    );
    conn.raw_cmd(&query).await.unwrap();

    Ok(())
}
