use crate::common::*;
use indoc::indoc;

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
            url = "sqlite://file.db"
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
              "value": "sqlite://file.db"
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

fn render_schema_json(schema: &str) -> String {
    let config = parse_configuration(schema);
    datamodel::json::mcf::render_sources_to_json(&config.datasources)
}
