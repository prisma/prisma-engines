use crate::common::*;
use indoc::indoc;

#[test]
fn missing_native_type_should_still_allow_config_parsing() {
    let schema = indoc! { r#"
        datasource db {
            provider = "postgresql"
            url  = env("DATABASE_URL")
        }

        model A {
            id Int @id
            val Int @db.
        }
    "#};

    parse_configuration(schema);
}

#[test]
fn serialize_builtin_sources_to_dmmf() {
    std::env::set_var("pg2", "postgresql://localhost/postgres2");

    let schema = indoc! { r#"
        datasource pg1 {
            provider = "postgresql"
            url = "postgresql://localhost/postgres1"
        }
    "#};

    let expect = expect![[r#"
        [
          {
            "name": "pg1",
            "provider": "postgresql",
            "activeProvider": "postgresql",
            "url": {
              "fromEnvVar": null,
              "value": "postgresql://localhost/postgres1"
            }
          }
        ]"#]];

    expect.assert_eq(&render_schema_json(schema));

    let schema = indoc! {r#"
        datasource pg2 {
            provider = "postgresql"
            url = env("pg2")
        }
    "#};

    let expect = expect![[r#"
        [
          {
            "name": "pg2",
            "provider": "postgresql",
            "activeProvider": "postgresql",
            "url": {
              "fromEnvVar": "pg2",
              "value": null
            }
          }
        ]"#]];

    expect.assert_eq(&render_schema_json(schema));

    let schema = indoc! {r#"
        datasource sqlite1 {
            provider = "sqlite"
            url = "file://file.db"
        }
    "#};

    let expect = expect![[r#"
        [
          {
            "name": "sqlite1",
            "provider": "sqlite",
            "activeProvider": "sqlite",
            "url": {
              "fromEnvVar": null,
              "value": "file://file.db"
            }
          }
        ]"#]];

    expect.assert_eq(&render_schema_json(schema));

    let schema = indoc! {r#"
        datasource mysql1 {
            provider = "mysql"
            url = "mysql://localhost"
        }
    "#};

    let expect = expect![[r#"
        [
          {
            "name": "mysql1",
            "provider": "mysql",
            "activeProvider": "mysql",
            "url": {
              "fromEnvVar": null,
              "value": "mysql://localhost"
            }
          }
        ]"#]];

    expect.assert_eq(&render_schema_json(schema));
}

#[test]
fn datasource_should_not_allow_arbitrary_parameters() {
    let schema = indoc! {r#"
        datasource ds {
          provider = "mysql"
          url = "mysql://localhost"
          foo = "bar"
        }
    "#};

    let expect = expect![[r#"
        [1;91merror[0m: [1mProperty not known: "foo".[0m
          [1;94m-->[0m  [4mschema.prisma:4[0m
        [1;94m   | [0m
        [1;94m 3 | [0m  url = "mysql://localhost"
        [1;94m 4 | [0m  [1;91mfoo = "bar"[0m
        [1;94m 5 | [0m}
        [1;94m   | [0m
    "#]];

    expect.assert_eq(&parse_unwrap_err(schema));
}

#[test]
fn unescaped_windows_paths_give_a_good_error() {
    let schema = indoc! {r#"
        datasource ds {
          provider = "sqlite"
          url = "file:c:\Windows32\data.db"
        }
    "#};

    let expect = expect![[r#"
        [1;91merror[0m: [1mUnknown escape sequence. If the value is a windows-style path, `\` must be escaped as `\\`.[0m
          [1;94m-->[0m  [4mschema.prisma:3[0m
        [1;94m   | [0m
        [1;94m 2 | [0m  provider = "sqlite"
        [1;94m 3 | [0m  url = "file:c:[1;91m\W[0mindows32\data.db"
        [1;94m   | [0m
        [1;91merror[0m: [1mUnknown escape sequence. If the value is a windows-style path, `\` must be escaped as `\\`.[0m
          [1;94m-->[0m  [4mschema.prisma:3[0m
        [1;94m   | [0m
        [1;94m 2 | [0m  provider = "sqlite"
        [1;94m 3 | [0m  url = "file:c:\Windows32[1;91m\d[0mata.db"
        [1;94m   | [0m
    "#]];

    expect.assert_eq(&parse_unwrap_err(schema));
}

#[test]
fn escaped_windows_paths_should_work() {
    let schema = indoc! {r#"
        datasource ds {
          provider = "sqlite"
          url = "file:c:\\Windows32\\data.db"
        }
    "#};

    assert!(datamodel::parse_schema(schema).is_ok());
}

fn render_schema_json(schema: &str) -> String {
    let config = parse_configuration(schema);
    datamodel::mcf::render_sources_to_json(&config.datasources)
}
