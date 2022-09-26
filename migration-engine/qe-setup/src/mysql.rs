use migration_core::migration_connector::{ConnectorError, ConnectorResult};
use test_setup::mysql::create_mysql_database;
use url::Url;

pub(crate) async fn mysql_reset(original_url: &str) -> ConnectorResult<()> {
    let url = Url::parse(original_url).map_err(ConnectorError::url_parse_error)?;
    let db_name = url.path().trim_start_matches('/');
    create_mysql_database(original_url, db_name).await.unwrap();
    Ok(())
}
