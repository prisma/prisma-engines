use migration_engine_tests::test_api::*;
use quaint::prelude::Queryable;

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

#[test_connector(tags(Sqlite))]
fn db_execute_happy_path_with_literal_url(api: TestApi) {
    let tmpdir = tempfile::TempDir::new().unwrap();
    let url = format!("file:{}/db1.sqlite", tmpdir.path().to_string_lossy());
    let script = r#"
        CREATE TABLE "dogs" ( id INTEGER PRIMARY KEY, name TEXT );
        INSERT INTO "dogs" ("name") VALUES ('snoopy'), ('marmaduke');
    "#;

    // Execute the command.
    api.db_execute(DbExecuteParams {
        datasource_type: DbExecuteDatasourceType::Url(UrlContainer { url: url.clone() }),
        script: script.to_owned(),
    })
    .unwrap();

    // Check that the command was executed
    let q = api.block_on(quaint::single::Quaint::new(&url)).unwrap();
    let result = api.block_on(q.query_raw("SELECT name FROM dogs;", &[])).unwrap();
    let mut rows = result.into_iter();
    assert_eq!(rows.next().unwrap()[0].to_string().unwrap(), "snoopy");
    assert_eq!(rows.next().unwrap()[0].to_string().unwrap(), "marmaduke");
}

#[test_connector(tags(Sqlite))]
fn db_execute_happy_path_with_prisma_schema(api: TestApi) {
    let tmpdir = tempfile::TempDir::new().unwrap();
    let url = format!("file:{}/dbfromschema.sqlite", tmpdir.path().to_string_lossy());
    let prisma_schema = format!(
        r#"
        datasource dbtest {{
            url = "{}"
            provider = "sqlite"
        }}
    "#,
        url
    );
    let schema_path = tmpdir.path().join("schema.prisma");
    std::fs::write(&schema_path, &prisma_schema).unwrap();
    let script = r#"
        CREATE TABLE "dogs" ( id INTEGER PRIMARY KEY, name TEXT );
        INSERT INTO "dogs" ("name") VALUES ('snoopy'), ('marmaduke');
    "#;

    // Execute the command.
    api.db_execute(DbExecuteParams {
        datasource_type: DbExecuteDatasourceType::Schema(SchemaContainer {
            schema: schema_path.to_string_lossy().into_owned(),
        }),
        script: script.to_owned(),
    })
    .unwrap();

    // Check that the command was executed
    let q = api.block_on(quaint::single::Quaint::new(&url)).unwrap();
    let result = api.block_on(q.query_raw("SELECT name FROM dogs;", &[])).unwrap();
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

    // Execute the command.
    api.db_execute(DbExecuteParams {
        datasource_type: DbExecuteDatasourceType::Url(UrlContainer { url: url.clone() }),
        script: script.to_owned(),
    })
    .unwrap();

    // Check that the command was executed
    let q = api.block_on(quaint::single::Quaint::new(&url)).unwrap();
    let result = api.block_on(q.query_raw("SELECT name FROM dogs;", &[])).unwrap();
    let mut rows = result.into_iter();
    assert_eq!(rows.next().unwrap()[0].to_string().unwrap(), "snoopy");
    assert_eq!(rows.next().unwrap()[0].to_string().unwrap(), "marmaduke");
}

#[test_connector(tags(Mysql))]
fn db_execute_error_path(api: TestApi) {
    let script = r#"
        CREATE TABLE "dogs" ( id INTEGER AUTO_INCREMENT PRIMARY KEY, name TEXT );
    "#;

    // Execute the command.
    let result = api.db_execute(DbExecuteParams {
        datasource_type: DbExecuteDatasourceType::Url(UrlContainer {
            url: api.connection_string().to_owned(),
        }),
        script: script.to_owned(),
    });

    assert!(result.is_err());
}
