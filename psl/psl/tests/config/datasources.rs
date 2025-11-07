use psl::builtin_connectors::PostgresDatasourceProperties;

use crate::common::*;

#[test]
fn missing_native_type_should_still_allow_config_parsing() {
    let schema = indoc! { r#"
        datasource db {
            provider = "postgresql"
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
    unsafe { std::env::set_var("pg2", "postgresql://localhost/postgres2") };

    let schema = indoc! { r#"
        datasource pg1 {
            provider = "postgresql"
        }
    "#};

    let expect = expect![[r#"
        [
          {
            "name": "pg1",
            "provider": "postgresql",
            "activeProvider": "postgresql",
            "schemas": [],
            "sourceFilePath": "schema.prisma"
          }
        ]"#]];

    expect.assert_eq(&render_datasources(schema));

    let schema = indoc! {r#"
        datasource pg2 {
            provider = "postgresql"
        }
    "#};

    let expect = expect![[r#"
        [
          {
            "name": "pg2",
            "provider": "postgresql",
            "activeProvider": "postgresql",
            "schemas": [],
            "sourceFilePath": "schema.prisma"
          }
        ]"#]];

    expect.assert_eq(&render_datasources(schema));

    let schema = indoc! {r#"
        datasource sqlite1 {
            provider = "sqlite"
        }
    "#};

    let expect = expect![[r#"
        [
          {
            "name": "sqlite1",
            "provider": "sqlite",
            "activeProvider": "sqlite",
            "schemas": [],
            "sourceFilePath": "schema.prisma"
          }
        ]"#]];

    expect.assert_eq(&render_datasources(schema));

    let schema = indoc! {r#"
        datasource mysql1 {
            provider = "mysql"
        }
    "#};

    let expect = expect![[r#"
        [
          {
            "name": "mysql1",
            "provider": "mysql",
            "activeProvider": "mysql",
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
          foo = "bar"
        }
    "#};

    let expect = expect![[r#"
        [1;91merror[0m: [1mProperty not known: "foo".[0m
          [1;94m-->[0m  [4mschema.prisma:3[0m
        [1;94m   | [0m
        [1;94m 2 | [0m  provider = "mysql"
        [1;94m 3 | [0m  [1;91mfoo = "bar"[0m
        [1;94m   | [0m
    "#]];

    expect.assert_eq(&parse_unwrap_err(schema));
}

#[test]
fn escaped_windows_paths_should_work() {
    let schema = indoc! {r#"
        datasource ds {
          provider = "sqlite"
        }
    "#};

    assert_valid(schema)
}

#[test]
fn postgresql_extension_parsing() {
    let schema = indoc! {r#"
        datasource ds {
          provider = "postgres"
          extensions = [postgis(version: "2.1", schema: "public"), uuidOssp(map: "uuid-ossp"), meow]
        }

        generator js {
          provider = "prisma-client"
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
          provider        = "prisma-client"
          previewFeatures = []
        }

        datasource ds {
          provider   = "postgres"
          schemas = []
        }
    "#};

    let expect = expect![[r#"
        [1;91merror[0m: [1mIf provided, the schemas array can not be empty.[0m
          [1;94m-->[0m  [4mschema.prisma:8[0m
        [1;94m   | [0m
        [1;94m 7 | [0m  provider   = "postgres"
        [1;94m 8 | [0m  schemas = [1;91m[][0m
        [1;94m   | [0m
    "#]];

    expect_error(schema, &expect);
}

#[test]
fn parse_direct_url_should_error() {
    let schema = indoc! {r#"
        generator js {
          provider        = "prisma-client"
        }

        datasource ds {
          provider   = "postgres"
          directUrl = env("DIRECT_DATABASE_URL")
        }
    "#};

    let expect = expect![[r#"
        [1;91merror[0m: [1mThe datasource property `directUrl` is no longer supported in schema files. Move connection URLs to `prisma.config.ts`. See https://pris.ly/d/config-datasource[0m
          [1;94m-->[0m  [4mschema.prisma:7[0m
        [1;94m   | [0m
        [1;94m 6 | [0m  provider   = "postgres"
        [1;94m 7 | [0m  [1;91mdirectUrl = env("DIRECT_DATABASE_URL")[0m
        [1;94m   | [0m
    "#]];

    expect_error(schema, &expect);
}

#[test]
fn parse_multi_schema_for_unsupported_connector_should_error() {
    let schema = indoc! {r#"
        generator js {
          provider        = "prisma-client"
        }

        datasource ds {
          provider   = "mysql"
          schemas = ["test"]
        }
    "#};

    let actual = parse_config(schema).unwrap_err();
    let expect = expect![[r#"
        [1;91merror[0m: [1mThe `schemas` property is not supported on the current connector.[0m
          [1;94m-->[0m  [4mschema.prisma:7[0m
        [1;94m   | [0m
        [1;94m 6 | [0m  provider   = "mysql"
        [1;94m 7 | [0m  schemas = [1;91m["test"][0m
        [1;94m   | [0m
    "#]];

    expect.assert_eq(&actual);
}
