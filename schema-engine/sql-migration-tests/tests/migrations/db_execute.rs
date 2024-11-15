use quaint::{prelude::Queryable, single::Quaint};
use schema_core::json_rpc::types::SchemasWithConfigDir;
use sql_migration_tests::test_api::*;
use sql_migration_tests::utils::to_schema_containers;
use sql_migration_tests::*;

#[test]
fn db_execute_input_source_takes_expected_json_shape() {
    let value = DbExecuteParams {
        datasource_type: DbExecuteDatasourceType::Url(UrlContainer {
            url: "uiuiui".to_owned(),
        }),
        script: "SQL goes here".to_owned(),
    };

    let expected = expect![[r#"
        {
          "datasourceType": {
            "tag": "url",
            "url": "uiuiui"
          },
          "script": "SQL goes here"
        }"#]];

    expected.assert_eq(&serde_json::to_string_pretty(&value).unwrap());
}

#[test]
fn db_execute_happy_path_with_literal_url() {
    let tmpdir = tempfile::TempDir::new().unwrap();
    let url = format!("file:{}/db1.sqlite", tmpdir.path().to_string_lossy());
    let script = r#"
        CREATE TABLE "dogs" ( id INTEGER PRIMARY KEY, name TEXT );
        INSERT INTO "dogs" ("name") VALUES ('snoopy'), ('marmaduke');
    "#;

    // Execute the command.
    let generic_api = schema_core::schema_api(None, None).unwrap();
    tok(generic_api.db_execute(DbExecuteParams {
        datasource_type: DbExecuteDatasourceType::Url(UrlContainer { url: url.clone() }),
        script: script.to_owned(),
    }))
    .unwrap();

    // Check that the command was executed
    let q = tok(quaint::single::Quaint::new(&url)).unwrap();
    let result = tok(q.query_raw("SELECT name FROM dogs;", &[])).unwrap();
    let mut rows = result.into_iter();
    assert_eq!(rows.next().unwrap()[0].to_string().unwrap(), "snoopy");
    assert_eq!(rows.next().unwrap()[0].to_string().unwrap(), "marmaduke");
}

#[test]
fn db_execute_happy_path_with_prisma_schema() {
    let tmpdir = tempfile::TempDir::new().unwrap();
    let url = format!("file:{}/dbfromschema.sqlite", tmpdir.path().to_string_lossy());
    let prisma_schema = format!(
        r#"
        datasource dbtest {{
            url = "{}"
            provider = "sqlite"
        }}
    "#,
        url.replace('\\', "\\\\")
    );
    let schema_path = tmpdir.path().join("schema.prisma");
    std::fs::write(&schema_path, prisma_schema.clone()).unwrap();
    let script = r#"
        CREATE TABLE "dogs" ( id INTEGER PRIMARY KEY, name TEXT );
        INSERT INTO "dogs" ("name") VALUES ('snoopy'), ('marmaduke');
    "#;

    // Execute the command.
    let generic_api = schema_core::schema_api(None, None).unwrap();
    tok(generic_api.db_execute(DbExecuteParams {
        datasource_type: DbExecuteDatasourceType::Schema(SchemasWithConfigDir {
            files: vec![SchemaContainer {
                path: schema_path.to_string_lossy().into_owned(),
                content: prisma_schema.to_string(),
            }],
            config_dir: schema_path.parent().unwrap().to_string_lossy().into_owned(),
        }),
        script: script.to_owned(),
    }))
    .unwrap();

    // Check that the command was executed
    let q = tok(quaint::single::Quaint::new(&url)).unwrap();
    let result = tok(q.query_raw("SELECT name FROM dogs;", &[])).unwrap();
    let mut rows = result.into_iter();
    assert_eq!(rows.next().unwrap()[0].to_string().unwrap(), "snoopy");
    assert_eq!(rows.next().unwrap()[0].to_string().unwrap(), "marmaduke");
}

#[test_connector(tags(Mysql))]
fn mysql_incomplete_script_works(api: TestApi) {
    let script = r#"
        CREATE TABLE `dogs` ( id INTEGER AUTO_INCREMENT PRIMARY KEY, name TEXT );
        INSERT INTO `dogs` (`name`) VALUES ('snoopy'), ('marmaduke') -- missing final semicolon
    "#;

    let url = api.connection_string().to_owned();
    let generic_api = schema_core::schema_api(None, None).unwrap();
    tok(generic_api.db_execute(DbExecuteParams {
        datasource_type: DbExecuteDatasourceType::Url(UrlContainer { url: url.clone() }),
        script: script.to_owned(),
    }))
    .unwrap();

    // Check that the command was executed
    let q = tok(Quaint::new(&url)).unwrap();
    let result = tok(q.query_raw("SELECT name FROM dogs;", &[])).unwrap();
    let mut rows = result.into_iter();
    assert_eq!(rows.next().unwrap()[0].to_string().unwrap(), "snoopy");
    assert_eq!(rows.next().unwrap()[0].to_string().unwrap(), "marmaduke");
}

#[test_connector(tags(Mysql))]
fn db_execute_error_path(api: TestApi) {
    let script = r#"
        -- wrong quotes
        CREATE TABLE "dogs" ( id INTEGER AUTO_INCREMENT PRIMARY KEY, name TEXT );
    "#;

    let generic_api = schema_core::schema_api(None, None).unwrap();
    let result = tok(generic_api.db_execute(DbExecuteParams {
        datasource_type: DbExecuteDatasourceType::Url(UrlContainer {
            url: api.connection_string().to_owned(),
        }),
        script: script.to_owned(),
    }));

    assert!(result.is_err());
}

#[test_connector(tags(Postgres12))]
fn db_execute_drop_database_that_doesnt_exist_error(api: TestApi) {
    let script = r#"
        DROP DATABASE "thisisadatabaseweassumedoesntexist";
    "#;

    let generic_api = schema_core::schema_api(None, None).unwrap();
    let result = tok(generic_api.db_execute(DbExecuteParams {
        datasource_type: DbExecuteDatasourceType::Url(UrlContainer {
            url: api.connection_string().to_owned(),
        }),
        script: script.to_owned(),
    }));

    let error = result.unwrap_err().to_string();
    let expectation = expect![[r#"
        Database `thisisadatabaseweassumedoesntexist` does not exist on the database server at `localhost:5434`.
    "#]];
    expectation.assert_eq(&error);
}

#[test]
fn sqlite_db_execute_with_schema_datasource_resolves_relative_paths_correctly() {
    let tmpdir = tempfile::tempdir().unwrap();
    let prisma_dir = tmpdir.path().join("prisma");
    std::fs::create_dir_all(&prisma_dir).unwrap();
    let schema_path = prisma_dir.join("schema.prisma");
    let schema = r#"
        datasource sqlitedb {
            provider = "sqlite"
            url = "file:./dev.db"
        }
    "#;
    std::fs::write(&schema_path, schema).unwrap();

    let expected_sqlite_path = prisma_dir.join("dev.db");
    assert!(!expected_sqlite_path.exists());

    let api = schema_core::schema_api(None, None).unwrap();
    tok(api.db_execute(DbExecuteParams {
        datasource_type: DbExecuteDatasourceType::Schema(SchemasWithConfigDir {
            files: vec![SchemaContainer {
                path: schema_path.to_str().unwrap().to_owned(),
                content: schema.to_owned(),
            }],
            config_dir: schema_path.parent().unwrap().to_string_lossy().into_owned(),
        }),
        script: "CREATE TABLE dog ( id INTEGER PRIMARY KEY )".to_owned(),
    }))
    .unwrap();

    assert!(expected_sqlite_path.exists());
}

#[test]
fn db_execute_multi_file() {
    let (tmpdir, files) = write_multi_file! {
        "a.prisma" => r#"
            datasource dbtest {
                provider = "sqlite"
                url = "file:db1.sqlite"
            }
        "#,
        "b.prisma" => r#"
            model dogs {
                id Int @id
            }
        "#,
    };

    let url = format!("file:{}/db1.sqlite", tmpdir.path().to_string_lossy());
    let script = r#"
        CREATE TABLE "dogs" ( id INTEGER PRIMARY KEY, name TEXT );
        INSERT INTO "dogs" ("name") VALUES ('snoopy'), ('marmaduke');
    "#;

    // Execute the command.
    let generic_api = schema_core::schema_api(None, None).unwrap();
    tok(generic_api.db_execute(DbExecuteParams {
        datasource_type: DbExecuteDatasourceType::Schema(SchemasWithConfigDir {
            files: to_schema_containers(&files),
            config_dir: tmpdir.path().to_string_lossy().into_owned(),
        }),
        script: script.to_owned(),
    }))
    .unwrap();

    // Check that the command was executed
    let q = tok(quaint::single::Quaint::new(&url)).unwrap();
    let result = tok(q.query_raw("SELECT name FROM dogs;", &[])).unwrap();
    let mut rows = result.into_iter();
    assert_eq!(rows.next().unwrap()[0].to_string().unwrap(), "snoopy");
    assert_eq!(rows.next().unwrap()[0].to_string().unwrap(), "marmaduke");
}
