use migration_core::migration_connector::{ConnectorError, ConnectorResult};
use url::Url;

pub(crate) async fn mongo_setup(schema: &str, url: &str) -> ConnectorResult<()> {
    let url = Url::parse(url).map_err(ConnectorError::url_parse_error).unwrap();
    let db_name = url.path().trim_start_matches('/').to_string();
    let client = mongodb_client::create(url).await.unwrap();

    client
        .database(&db_name)
        .drop(Some(
            mongodb::options::DropDatabaseOptions::builder()
                .write_concern(mongodb::options::WriteConcern::builder().journal(true).build())
                .build(),
        ))
        .await
        .unwrap();

    let parsed_schema = datamodel::parse_schema_parserdb(schema).unwrap();

    for model in parsed_schema.db.walk_models() {
        client
            .database(&db_name)
            .create_collection(model.database_name(), None)
            .await
            .unwrap();
    }

    Ok(())
}
