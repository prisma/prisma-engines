use std::path::Path;

use crate::common::parse;
use indoc::indoc;
use pretty_assertions::assert_eq;

fn from_env(key: &str) -> Option<String> {
    std::env::var(key).ok()
}

#[test]
fn strings_with_quotes_are_unescaped() {
    let input = indoc!(
        r#"
        model Category {
          id   String @id
          name String @default("a \" b\"c d")
        }"#
    );

    let mut dml = parse(input);
    let cat = dml.models_mut().find(|m| m.name == "Category").unwrap();
    let name = cat.scalar_fields().find(|f| f.name == "name").unwrap();

    assert_eq!(
        name.default_value
            .as_ref()
            .unwrap()
            .as_single()
            .unwrap()
            .as_string()
            .unwrap(),
        "a \" b\"c d"
    );
}

#[test]
fn strings_with_newlines_are_unescpaed() {
    let input = indoc!(
        r#"
        model Category {
          id   String @id
          name String @default("Jean\nClaude\nVan\nDamme")
        }"#
    );

    let mut dml = parse(input);
    let cat = dml.models_mut().find(|m| m.name == "Category").unwrap();
    let name = cat.scalar_fields().find(|f| f.name == "name").unwrap();

    assert_eq!(
        name.default_value
            .as_ref()
            .unwrap()
            .as_single()
            .unwrap()
            .as_string()
            .unwrap(),
        "Jean\nClaude\nVan\nDamme"
    );
}

#[test]
fn relative_sqlite_paths_can_be_modified() {
    let schema = indoc!(
        r#"
        datasource boo {
          provider = "sqlite"
          url = "file:dev.db"
        }"#
    );

    let config = datamodel::parse_configuration(schema).unwrap();
    let url = config.subject.datasources[0].load_url_with_config_dir(Path::new("/path/to/prisma"), from_env);

    assert_eq!("file:/path/to/prisma/dev.db", url.unwrap())
}

#[test]
fn absolute_sqlite_paths_are_not_modified() {
    let schema = indoc!(
        r#"
        datasource boo {
          provider = "sqlite"
          url = "file:/foo/bar/dev.db"
        }"#
    );

    let config = datamodel::parse_configuration(schema).unwrap();
    let url = config.subject.datasources[0].load_url_with_config_dir(Path::new("/path/to/prisma"), from_env);

    assert_eq!("file:/foo/bar/dev.db", url.unwrap())
}

#[test]
fn postgres_relative_sslidentity_can_be_modified() {
    let schema = indoc!(
        r#"
        datasource boo {
          provider = "postgres"
          url = "postgres://localhost:420/?foo=bar&sslidentity=we%2Fare%2Fhere.key"
        }"#
    );

    let config = datamodel::parse_configuration(schema).unwrap();
    let url = config.subject.datasources[0].load_url_with_config_dir(Path::new("/path/to/prisma"), from_env);

    assert_eq!(
        "postgres://localhost:420/?foo=bar&sslidentity=%2Fpath%2Fto%2Fprisma%2Fwe%2Fare%2Fhere.key",
        url.unwrap()
    )
}

#[test]
fn postgres_absolute_sslidentity_should_not_be_modified() {
    let schema = indoc!(
        r#"
        datasource boo {
          provider = "postgres"
          url = "postgres://localhost:420/?foo=bar&sslidentity=%2Fwe%2Fare%2Fhere.key"
        }"#
    );

    let config = datamodel::parse_configuration(schema).unwrap();
    let url = config.subject.datasources[0]
        .load_url_with_config_dir(Path::new("/path/to/prisma"), from_env)
        .unwrap();

    assert_eq!(
        "postgres://localhost:420/?foo=bar&sslidentity=%2Fwe%2Fare%2Fhere.key",
        url
    )
}

#[test]
fn mysql_relative_sslidentity_can_be_modified() {
    let schema = indoc!(
        r#"
        datasource boo {
          provider = "mysql"
          url = "mysql://localhost:420/?foo=bar&sslidentity=we%2Fare%2Fhere.key"
        }"#
    );

    let config = datamodel::parse_configuration(schema).unwrap();
    let url = config.subject.datasources[0]
        .load_url_with_config_dir(Path::new("/path/to/prisma"), from_env)
        .unwrap();

    assert_eq!(
        "mysql://localhost:420/?foo=bar&sslidentity=%2Fpath%2Fto%2Fprisma%2Fwe%2Fare%2Fhere.key",
        url
    )
}

#[test]
fn mysql_absolute_sslidentity_should_not_be_modified() {
    let schema = indoc!(
        r#"
        datasource boo {
          provider = "mysql"
          url = "mysql://localhost:420/?foo=bar&sslidentity=%2Fwe%2Fare%2Fhere.key"
        }"#
    );

    let config = datamodel::parse_configuration(schema).unwrap();
    let url = config.subject.datasources[0]
        .load_url_with_config_dir(Path::new("/path/to/prisma"), from_env)
        .unwrap();

    assert_eq!("mysql://localhost:420/?foo=bar&sslidentity=%2Fwe%2Fare%2Fhere.key", url)
}

#[test]
fn postgres_relative_sslcert_can_be_modified() {
    let schema = indoc!(
        r#"
        datasource boo {
          provider = "postgres"
          url = "postgres://localhost:420/?foo=bar&sslcert=we%2Fare%2Fhere.crt"
        }"#
    );

    let config = datamodel::parse_configuration(schema).unwrap();
    let url = config.subject.datasources[0]
        .load_url_with_config_dir(Path::new("/path/to/prisma"), from_env)
        .unwrap();

    assert_eq!(
        "postgres://localhost:420/?foo=bar&sslcert=%2Fpath%2Fto%2Fprisma%2Fwe%2Fare%2Fhere.crt",
        url
    )
}

#[test]
fn postgres_absolute_sslcert_should_not_be_modified() {
    let schema = indoc!(
        r#"
        datasource boo {
          provider = "postgres"
          url = "postgres://localhost:420/?foo=bar&sslcert=%2Fwe%2Fare%2Fhere.crt"
        }"#
    );

    let config = datamodel::parse_configuration(schema).unwrap();
    let url = config.subject.datasources[0]
        .load_url_with_config_dir(Path::new("/path/to/prisma"), from_env)
        .unwrap();

    assert_eq!("postgres://localhost:420/?foo=bar&sslcert=%2Fwe%2Fare%2Fhere.crt", url)
}

#[test]
fn mysql_relative_sslcert_can_be_modified() {
    let schema = indoc!(
        r#"
        datasource boo {
          provider = "mysql"
          url = "mysql://localhost:420/?foo=bar&sslcert=we%2Fare%2Fhere.crt"
        }"#
    );

    let config = datamodel::parse_configuration(schema).unwrap();
    let url = config.subject.datasources[0]
        .load_url_with_config_dir(Path::new("/path/to/prisma"), from_env)
        .unwrap();

    assert_eq!(
        "mysql://localhost:420/?foo=bar&sslcert=%2Fpath%2Fto%2Fprisma%2Fwe%2Fare%2Fhere.crt",
        url
    )
}

#[test]
fn mysql_absolute_sslcert_should_not_be_modified() {
    let schema = indoc!(
        r#"
        datasource boo {
          provider = "mysql"
          url = "mysql://localhost:420/?foo=bar&sslcert=%2Fwe%2Fare%2Fhere.crt"
        }"#
    );

    let config = datamodel::parse_configuration(schema).unwrap();
    let url = config.subject.datasources[0]
        .load_url_with_config_dir(Path::new("/path/to/prisma"), from_env)
        .unwrap();

    assert_eq!("mysql://localhost:420/?foo=bar&sslcert=%2Fwe%2Fare%2Fhere.crt", url)
}
