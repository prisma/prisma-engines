use crate::AnyError;
use quaint::{prelude::Queryable, single::Quaint};
use url::Url;

/// Returns a connection string for an isolated Kingbase MySQL-compatible test database.
pub async fn create_kingbase_mysql_database(database_url: &str, db_name: &str) -> Result<(String, String), AnyError> {
    let mut url: Url = database_url.parse()?;
    let mut maintenance_url = url.clone();
    let current_database = url.path().trim_start_matches('/');
    let prefix = std::env::var("TEST_KINGBASE_DATABASE_PREFIX").unwrap_or_default();
    if prefix.len() > 16 || !prefix.bytes().all(|byte| byte.is_ascii_alphanumeric() || byte == b'_') {
        return Err(
            "TEST_KINGBASE_DATABASE_PREFIX must contain at most 16 ASCII letters, digits or underscores".into(),
        );
    }
    let database_name = format!("{prefix}{db_name}");
    let db_name = kingbase_safe_identifier(&database_name);

    let maintenance_database = if current_database.is_empty() || current_database.eq_ignore_ascii_case("test") {
        "template1"
    } else {
        "test"
    };

    maintenance_url.set_path(&format!("/{maintenance_database}"));
    let query = url
        .query_pairs()
        .filter(|(key, _)| key != "schema")
        .map(|(key, value)| (key.into_owned(), value.into_owned()))
        .collect::<Vec<_>>();
    maintenance_url
        .query_pairs_mut()
        .clear()
        .extend_pairs(query.iter().map(|(key, value)| (key.as_str(), value.as_str())));
    url.set_path(db_name);

    let conn = Quaint::new(maintenance_url.as_ref()).await?;
    conn.raw_cmd(&format!("DROP DATABASE IF EXISTS `{db_name}`")).await?;
    conn.raw_cmd(&format!("CREATE DATABASE `{db_name}`")).await?;

    Ok((db_name.to_owned(), url.to_string()))
}

fn kingbase_safe_identifier(identifier: &str) -> &str {
    const MAX_IDENTIFIER_BYTES: usize = 63;

    if identifier.len() <= MAX_IDENTIFIER_BYTES {
        return identifier;
    }

    let end = identifier
        .char_indices()
        .map(|(index, _)| index)
        .take_while(|index| *index <= MAX_IDENTIFIER_BYTES)
        .last()
        .unwrap_or_default();

    &identifier[..end]
}
