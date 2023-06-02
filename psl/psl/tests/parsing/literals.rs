use crate::common::*;
use std::path::Path;

fn from_env(key: &str) -> Option<String> {
    std::env::var(key).ok()
}

#[test]
fn strings_with_quotes_are_unescaped() {
    let input = indoc! {r#"
        model Category {
          id   String @id
          name String @default("a \" b\"c d")
        }
    "#};

    psl::parse_schema(input)
        .unwrap()
        .assert_has_model("Category")
        .assert_has_scalar_field("name")
        .assert_default_value()
        .assert_string("a \" b\"c d");
}

#[test]
fn strings_with_newlines_are_unescaped() {
    let input = indoc! {r#"
        model Category {
          id   String @id
          name String @default("Jean\nClaude\nVan\nDamme")
        }
    "#};

    psl::parse_schema(input)
        .unwrap()
        .assert_has_model("Category")
        .assert_has_scalar_field("name")
        .assert_default_value()
        .assert_string("Jean\nClaude\nVan\nDamme");
}

#[test]
fn strings_with_escaped_unicode_codepoints_are_unescaped() {
    let input = indoc! {r#"
        model Category {
          id   String @id
          name String @default("mfw \u56e7 - \u56E7 ^^")
          // Escaped UTF-16 with surrogate pair (rolling eyes emoji).
          nameUtf16 String @default("oh my \ud83d\ude44...")
        }
    "#};

    let dml = psl::parse_schema(input).unwrap();
    let cat = dml.assert_has_model("Category");

    cat.assert_has_scalar_field("name")
        .assert_default_value()
        .assert_string("mfw 囧 - 囧 ^^");

    cat.assert_has_scalar_field("nameUtf16")
        .assert_default_value()
        .assert_string("oh my 🙄...");
}

#[test]
fn string_literals_with_invalid_unicode_escapes() {
    let input = indoc!(
        r#"
        model Category {
          id   String @id
          name String @default("Something \uD802 \ut \u12")
        }"#
    );

    let expectation = expect![[r#"
        [1;91merror[0m: [1mInvalid unicode escape sequence.[0m
          [1;94m-->[0m  [4mschema.prisma:3[0m
        [1;94m   | [0m
        [1;94m 2 | [0m  id   String @id
        [1;94m 3 | [0m  name String @default("Something [1;91m\uD802[0m \ut \u12")
        [1;94m   | [0m
        [1;91merror[0m: [1mInvalid unicode escape sequence.[0m
          [1;94m-->[0m  [4mschema.prisma:3[0m
        [1;94m   | [0m
        [1;94m 2 | [0m  id   String @id
        [1;94m 3 | [0m  name String @default("Something \uD802 [1;91m\u[0mt \u12")
        [1;94m   | [0m
        [1;91merror[0m: [1mInvalid unicode escape sequence.[0m
          [1;94m-->[0m  [4mschema.prisma:3[0m
        [1;94m   | [0m
        [1;94m 2 | [0m  id   String @id
        [1;94m 3 | [0m  name String @default("Something \uD802 \ut [1;91m\u12[0m")
        [1;94m   | [0m
    "#]];

    expect_error(input, &expectation);
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

    let config = parse_configuration(schema);
    let url = config.datasources[0].load_url_with_config_dir(Path::new("/path/to/prisma"), from_env);

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

    let config = parse_configuration(schema);
    let url = config.datasources[0].load_url_with_config_dir(Path::new("/path/to/prisma"), from_env);

    assert_eq!("file:/foo/bar/dev.db", url.unwrap())
}

#[test]
fn mongo_relative_tlscafile_can_be_modified() {
    let schema = indoc!(
        r#"
        datasource boo {
          provider = "mongodb"
          url = "mongodb://localhost:420/?foo=bar&tlsCAFile=we%2Fare%2Fhere.key"
        }"#
    );

    let config = parse_configuration(schema);
    let url = config.datasources[0].load_url_with_config_dir(Path::new("/path/to/prisma"), from_env);

    assert_eq!(
        "mongodb://localhost:420/?foo=bar&tlsCAFile=%2Fpath%2Fto%2Fprisma%2Fwe%2Fare%2Fhere.key",
        url.unwrap()
    )
}

#[test]
fn mongo_absolute_tlscafile_should_not_be_modified() {
    let schema = indoc!(
        r#"
        datasource boo {
          provider = "mongodb"
          url = "mongodb://localhost:420/?foo=bar&tlsCAFile=%2Fwe%2Fare%2Fhere.key"
        }"#
    );

    let config = parse_configuration(schema);
    let url = config.datasources[0]
        .load_url_with_config_dir(Path::new("/path/to/prisma"), from_env)
        .unwrap();

    assert_eq!("mongodb://localhost:420/?foo=bar&tlsCAFile=%2Fwe%2Fare%2Fhere.key", url)
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

    let config = parse_configuration(schema);
    let url = config.datasources[0].load_url_with_config_dir(Path::new("/path/to/prisma"), from_env);

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

    let config = parse_configuration(schema);
    let url = config.datasources[0]
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

    let config = parse_configuration(schema);
    let url = config.datasources[0]
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

    let config = parse_configuration(schema);
    let url = config.datasources[0]
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

    let config = parse_configuration(schema);
    let url = config.datasources[0]
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

    let config = parse_configuration(schema);
    let url = config.datasources[0]
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

    let config = parse_configuration(schema);
    let url = config.datasources[0]
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

    let config = parse_configuration(schema);
    let url = config.datasources[0]
        .load_url_with_config_dir(Path::new("/path/to/prisma"), from_env)
        .unwrap();

    assert_eq!("mysql://localhost:420/?foo=bar&sslcert=%2Fwe%2Fare%2Fhere.crt", url)
}

#[test]
fn sql_server_relative_ca_file_can_be_modified() {
    let schema = indoc!(
        r#"
        datasource boo {
          provider = "sqlserver"
          url = "sqlserver://localhost:1433;trustServerCertificateCA=customCA.crt"
        }"#
    );

    let config = parse_configuration(schema);
    let url = config.datasources[0].load_url_with_config_dir(Path::new("/path/to/prisma"), from_env);

    assert_eq!(
        "sqlserver://localhost:1433;trustServerCertificateCA={/}path{/}to{/}prisma{/}customCA.crt",
        url.unwrap()
    )
}

#[test]
fn sql_server_absolute_ca_file_should_not_be_modified() {
    let schema = indoc!(
        r#"
        datasource boo {
          provider = "sqlserver"
          url = "sqlserver://localhost:1433;trustServerCertificateCA={/}foo{/}bar{/}customCA.crt"
        }"#
    );

    let config = parse_configuration(schema);
    let url = config.datasources[0]
        .load_url_with_config_dir(Path::new("/path/to/prisma"), from_env)
        .unwrap();

    assert_eq!(
        "sqlserver://localhost:1433;trustServerCertificateCA={/}foo{/}bar{/}customCA.crt",
        url
    )
}

#[test]
fn sql_server_absolute_windows_ca_file_should_not_be_modified() {
    let schema = indoc!(
        r#"
        datasource boo {
          provider = "sqlserver"
          url = "sqlserver://localhost:1433;trustServerCertificateCA=C:{\\\\}path{\\}customCA.crt"
        }"#
    );

    let config = parse_configuration(schema);
    let url = config.datasources[0]
        .load_url_with_config_dir(Path::new("/path/to/prisma"), from_env)
        .unwrap();

    assert_eq!(
        r#"sqlserver://localhost:1433;trustServerCertificateCA=C:{\\}path{\}customCA.crt"#,
        url
    )
}
