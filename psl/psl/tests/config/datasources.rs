use psl::builtin_connectors::PostgresDatasourceProperties;

use crate::common::*;

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
            },
            "schemas": [],
            "sourceFilePath": "schema.prisma"
          }
        ]"#]];

    expect.assert_eq(&render_datasources(schema));

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
            },
            "schemas": [],
            "sourceFilePath": "schema.prisma"
          }
        ]"#]];

    expect.assert_eq(&render_datasources(schema));

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
            },
            "schemas": [],
            "sourceFilePath": "schema.prisma"
          }
        ]"#]];

    expect.assert_eq(&render_datasources(schema));

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
            },
            "schemas": [],
            "sourceFilePath": "schema.prisma"
          }
        ]"#]];

    expect.assert_eq(&render_datasources(schema));
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

    assert_valid(schema)
}

#[test]
fn postgresql_extension_parsing() {
    let schema = indoc! {r#"
        datasource ds {
          provider = "postgres"
          url = env("DATABASE_URL")
          extensions = [postgis(version: "2.1", schema: "public"), uuidOssp(map: "uuid-ossp"), meow]
        }

        generator js {
          provider = "prisma-client-js"
          previewFeatures = ["postgresqlExtensions"]
        }
    "#};

    let config = psl::parse_configuration(schema).unwrap();
    let properties: &PostgresDatasourceProperties =
        config.datasources.first().unwrap().downcast_connector_data().unwrap();

    assert!(properties.extensions().is_some());

    let mut extensions = properties.extensions().unwrap().extensions().iter();

    let meow = extensions.next().unwrap();
    assert_eq!("meow", meow.name());
    assert_eq!(None, meow.db_name());
    assert_eq!(None, meow.version());
    assert_eq!(None, meow.schema());

    let postgis = extensions.next().unwrap();
    assert_eq!("postgis", postgis.name());
    assert_eq!(None, postgis.db_name());
    assert_eq!(Some("2.1"), postgis.version());
    assert_eq!(Some("public"), postgis.schema());

    let uuid_ossp = extensions.next().unwrap();
    assert_eq!("uuidOssp", uuid_ossp.name());
    assert_eq!(Some("uuid-ossp"), uuid_ossp.db_name());
    assert_eq!(None, uuid_ossp.version());
    assert_eq!(None, uuid_ossp.schema());
}

#[test]
fn empty_schema_property_should_error() {
    let schema = indoc! {r#"
        generator js {
          provider        = "prisma-client-js"
          previewFeatures = ["multiSchema"]
        }

        datasource ds {
          provider   = "postgres"
          url        = env("DATABASE_URL")
          schemas = []
        }
    "#};

    let expect = expect![[r#"
        [1;91merror[0m: [1mIf provided, the schemas array can not be empty.[0m
          [1;94m-->[0m  [4mschema.prisma:9[0m
        [1;94m   | [0m
        [1;94m 8 | [0m  url        = env("DATABASE_URL")
        [1;94m 9 | [0m  schemas = [1;91m[][0m
        [1;94m   | [0m
    "#]];

    expect_error(schema, &expect);
}

#[test]
fn schemas_array_without_preview_feature_should_error() {
    let schema = indoc! {r#"
        generator js {
          provider        = "prisma-client-js"
        }

        datasource ds {
          provider   = "postgres"
          url        = env("DATABASE_URL")
          schemas = ["test"]
        }
    "#};

    let expect = expect![[r#"
        [1;91merror[0m: [1mThe `schemas` property is only availably with the `multiSchema` preview feature.[0m
          [1;94m-->[0m  [4mschema.prisma:8[0m
        [1;94m   | [0m
        [1;94m 7 | [0m  url        = env("DATABASE_URL")
        [1;94m 8 | [0m  schemas = [1;91m["test"][0m
        [1;94m   | [0m
    "#]];

    expect_error(schema, &expect);
}

#[test]
fn parse_direct_url_should_work() {
    let schema = indoc! {r#"
        generator js {
          provider        = "prisma-client-js"
        }

        datasource ds {
          provider   = "postgres"
          url        = env("DATABASE_URL")
          directUrl = env("DIRECT_DATABASE_URL")
        }
    "#};

    // assert_valid(schema);
    let config = parse_configuration(schema);

    let result = config
        .datasources
        .first()
        .and_then(|ds| ds.direct_url.clone())
        .and_then(|url| url.from_env_var)
        .unwrap();

    assert_eq!("DIRECT_DATABASE_URL", result);
}
