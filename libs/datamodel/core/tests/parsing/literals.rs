use std::path::Path;

use crate::common::parse;
use indoc::indoc;
use pretty_assertions::assert_eq;

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
            .get()
            .unwrap()
            .into_string()
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
            .get()
            .unwrap()
            .into_string()
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

    let config =
        datamodel::parse_configuration_with_config_dir(schema, Vec::new(), &Path::new("/path/to/prisma")).unwrap();

    assert_eq!("file:/path/to/prisma/dev.db", &config.subject.datasources[0].url.value)
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

    let config =
        datamodel::parse_configuration_with_config_dir(schema, Vec::new(), &Path::new("/path/to/prisma")).unwrap();

    assert_eq!("file:/foo/bar/dev.db", &config.subject.datasources[0].url.value)
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

    let config =
        datamodel::parse_configuration_with_config_dir(schema, Vec::new(), &Path::new("/path/to/prisma")).unwrap();

    assert_eq!(
        "postgres://localhost:420/?foo=bar&sslidentity=%2Fpath%2Fto%2Fprisma%2Fwe%2Fare%2Fhere.key",
        &config.subject.datasources[0].url.value
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

    let config =
        datamodel::parse_configuration_with_config_dir(schema, Vec::new(), &Path::new("/path/to/prisma")).unwrap();

    assert_eq!(
        "postgres://localhost:420/?foo=bar&sslidentity=%2Fwe%2Fare%2Fhere.key",
        &config.subject.datasources[0].url.value
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

    let config =
        datamodel::parse_configuration_with_config_dir(schema, Vec::new(), &Path::new("/path/to/prisma")).unwrap();

    assert_eq!(
        "mysql://localhost:420/?foo=bar&sslidentity=%2Fpath%2Fto%2Fprisma%2Fwe%2Fare%2Fhere.key",
        &config.subject.datasources[0].url.value
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

    let config =
        datamodel::parse_configuration_with_config_dir(schema, Vec::new(), &Path::new("/path/to/prisma")).unwrap();

    assert_eq!(
        "mysql://localhost:420/?foo=bar&sslidentity=%2Fwe%2Fare%2Fhere.key",
        &config.subject.datasources[0].url.value
    )
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

    let config =
        datamodel::parse_configuration_with_config_dir(schema, Vec::new(), &Path::new("/path/to/prisma")).unwrap();

    assert_eq!(
        "postgres://localhost:420/?foo=bar&sslcert=%2Fpath%2Fto%2Fprisma%2Fwe%2Fare%2Fhere.crt",
        &config.subject.datasources[0].url.value
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

    let config =
        datamodel::parse_configuration_with_config_dir(schema, Vec::new(), &Path::new("/path/to/prisma")).unwrap();

    assert_eq!(
        "postgres://localhost:420/?foo=bar&sslcert=%2Fwe%2Fare%2Fhere.crt",
        &config.subject.datasources[0].url.value
    )
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

    let config =
        datamodel::parse_configuration_with_config_dir(schema, Vec::new(), &Path::new("/path/to/prisma")).unwrap();

    assert_eq!(
        "mysql://localhost:420/?foo=bar&sslcert=%2Fpath%2Fto%2Fprisma%2Fwe%2Fare%2Fhere.crt",
        &config.subject.datasources[0].url.value
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

    let config =
        datamodel::parse_configuration_with_config_dir(schema, Vec::new(), &Path::new("/path/to/prisma")).unwrap();

    assert_eq!(
        "mysql://localhost:420/?foo=bar&sslcert=%2Fwe%2Fare%2Fhere.crt",
        &config.subject.datasources[0].url.value
    )
}
