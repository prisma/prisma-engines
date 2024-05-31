use connection_string::JdbcString;
use expect_test::expect;
use indoc::*;
use schema_core::json_rpc::types::*;
use std::{
    fs,
    io::{BufRead, BufReader, Write as _},
    panic::{self, AssertUnwindSafe},
    process::{Child, Command, Output},
};
use test_macros::test_connector;
use test_setup::{runtime::run_with_thread_local_runtime as tok, BitFlags, Tags, TestApiArgs};
use url::Url;
use user_facing_errors::{common::DatabaseDoesNotExist, UserFacingError};

fn schema_engine_bin_path() -> &'static str {
    env!("CARGO_BIN_EXE_schema-engine")
}

fn run(args: &[&str]) -> Output {
    Command::new(schema_engine_bin_path())
        .arg("cli")
        .args(args)
        .env("RUST_LOG", "INFO")
        .output()
        .unwrap()
}

fn with_child_process<F>(mut command: Command, f: F)
where
    F: FnOnce(&mut Child),
{
    let mut child = command
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()
        .unwrap();

    let res = panic::catch_unwind(AssertUnwindSafe(|| {
        f(&mut child);
    }));

    child.kill().unwrap();
    match res {
        Ok(_) => (),
        Err(panic_payload) => {
            let res = panic_payload
                .downcast_ref::<&str>()
                .map(|s| -> String { (*s).to_owned() })
                .or_else(|| panic_payload.downcast_ref::<String>().map(|s| s.to_owned()))
                .unwrap_or_default();

            panic!("Error: '{}'", res)
        }
    }
}

struct TestApi {
    args: TestApiArgs,
}

impl TestApi {
    fn new(args: TestApiArgs) -> Self {
        TestApi { args }
    }

    fn connection_string(&self) -> String {
        let args = &self.args;

        if args.tags().contains(Tags::Postgres) {
            tok(args.create_postgres_database()).2
        } else if args.tags().contains(Tags::Mysql) {
            tok(args.create_mysql_database()).1
        } else if args.tags().contains(Tags::Mssql) {
            tok(args.create_mssql_database()).1
        } else if args.tags().contains(Tags::Sqlite) {
            args.database_url().to_owned()
        } else {
            unreachable!()
        }
    }

    fn run(&self, args: &[&str]) -> Output {
        run(args)
    }
}

macro_rules! write_multi_file_vec {
    // Match multiple pairs of filename and content
    ( $( $filename:expr => $content:expr ),* $(,)? ) => {
        {
            use std::fs::File;
            use std::io::Write;

            // Create a result vector to collect errors
            let mut results = Vec::new();
            let tmpdir = tempfile::tempdir().unwrap();

            fs::create_dir_all(&tmpdir).unwrap();

            $(
                let file_path = tmpdir.path().join($filename);
                // Attempt to create or open the file
                let result = (|| -> std::io::Result<()> {
                    let mut file = File::create(&file_path)?;
                    file.write_all($content.as_bytes())?;
                    Ok(())
                })();

                result.unwrap();

                // Push the result of the operation to the results vector
                results.push((file_path.to_string_lossy().into_owned(), $content));
            )*

            // Return the results vector for further inspection if needed
            (tmpdir, results)
        }
    };
  }

#[test_connector(tags(Mysql))]
fn test_connecting_with_a_working_mysql_connection_string(api: TestApi) {
    let connection_string = api.connection_string();
    let output = api.run(&["--datasource", &connection_string, "can-connect-to-database"]);

    assert!(output.status.success(), "{output:?}");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Connection successful"), "{stderr:?}");
}

#[test_connector(tags(Mysql))]
fn test_connecting_with_a_non_working_mysql_connection_string(api: TestApi) {
    let mut non_existing_url: url::Url = api.args.database_url().parse().unwrap();

    non_existing_url.set_path("this_does_not_exist");

    let output = api.run(&["--datasource", non_existing_url.as_ref(), "can-connect-to-database"]);
    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains(r#""error_code":"P1003""#), "{}", stderr);
}

#[test_connector(tags(Postgres))]
fn test_connecting_with_a_working_postgres_connection_string(api: TestApi) {
    let conn_string = if api.args.database_url().starts_with("postgres:") {
        api.args.database_url().replacen("postgres:", "postgresql:", 1)
    } else {
        api.args.database_url().to_owned()
    };

    let output = api.run(&["--datasource", &conn_string, "can-connect-to-database"]);

    assert!(output.status.success(), "{output:?}");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Connection successful"), "{stderr:?}");
}

// Note: not redundant with previous test because of the different URL scheme.
#[test_connector(tags(Postgres))]
fn test_connecting_with_a_working_postgresql_connection_string(api: TestApi) {
    let conn_string = if api.args.database_url().starts_with("postgresql:") {
        api.args.database_url().replacen("postgresql:", "postgres:", 1)
    } else {
        api.args.database_url().to_owned()
    };

    let output = api.run(&["--datasource", &conn_string, "can-connect-to-database"]);

    assert!(output.status.success(), "{output:?}");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Connection successful"), "{stderr:?}");
}

#[test_connector(tags(Postgres))]
fn test_connecting_with_a_non_working_psql_connection_string(api: TestApi) {
    let mut url: url::Url = api.args.database_url().parse().unwrap();
    url.set_path("this_does_not_exist");

    let output = api.run(&["--datasource", url.as_ref(), "can-connect-to-database"]);
    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains(r#""error_code":"P1003""#), "{}", stderr);
}

#[test_connector(tags(Mssql))]
fn test_connecting_with_a_working_mssql_connection_string(api: TestApi) {
    let connection_string = api.connection_string();

    let output = api.run(&["--datasource", &connection_string, "can-connect-to-database"]);

    assert!(output.status.success(), "{output:?}");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Connection successful"), "{stderr:?}");
}

#[test_connector(tags(Postgres, Mysql))]
fn test_create_database(api: TestApi) {
    let connection_string = api.connection_string();
    let output = api.run(&["--datasource", &connection_string, "drop-database"]);
    assert!(output.status.success(), "{output:#?}");

    let output = api.run(&["--datasource", &connection_string, "create-database"]);
    assert!(output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Database 'test_create_database\' was successfully created."));

    let output = api.run(&["--datasource", &connection_string, "can-connect-to-database"]);
    assert!(output.status.success());
}

#[test_connector(tags(Mssql))]
fn test_create_database_mssql(api: TestApi) {
    let connection_string = api
        .connection_string()
        .replace("test_create_database_mssql", "test_create_database_NEW");

    let output = api.run(&["--datasource", &connection_string, "drop-database"]);
    assert!(output.status.success());

    let output = api.run(&["--datasource", &connection_string, "create-database"]);
    assert!(output.status.success());

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Database 'test_create_database_NEW\' was successfully created."));

    let output = api.run(&["--datasource", &connection_string, "can-connect-to-database"]);
    assert!(output.status.success());
}

#[test_connector(tags(Sqlite))]
fn test_sqlite_url(api: TestApi) {
    let base_dir = tempfile::tempdir().unwrap();
    let sqlite_path = base_dir.path().join("test.db");
    let url = format!("{}", sqlite_path.to_string_lossy());
    let output = api.run(&["--datasource", &url, "can-connect-to-database"]);
    assert!(!output.status.success());
    let message = String::from_utf8(output.stderr).unwrap();
    assert!(message.contains("The provided database string is invalid. The scheme is not recognized in database URL."));
}

#[test_connector(tags(Sqlite))]
fn test_create_sqlite_database(api: TestApi) {
    let base_dir = tempfile::tempdir().unwrap();

    let sqlite_path = base_dir
        .path()
        .join("doesntexist/either")
        .join("test_create_sqlite_database.db");

    assert!(!sqlite_path.exists());

    let url = format!("file:{}", sqlite_path.to_string_lossy());
    let output = api.run(&["--datasource", &url, "create-database"]);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success(), "{stderr:?}");
    assert!(stderr.contains("success"));
    assert!(stderr.contains("test_create_sqlite_database.db"));

    assert!(sqlite_path.exists());
}

#[test_connector(tags(Sqlite))]
fn test_drop_sqlite_database(api: TestApi) {
    let base_dir = tempfile::tempdir().unwrap();
    let sqlite_path = base_dir.path().join("test.db");
    let url = format!("file:{}", sqlite_path.to_string_lossy());

    let output = api.run(&["--datasource", &url, "create-database"]);
    assert!(output.status.success());
    let output = api.run(&["--datasource", &url, "can-connect-to-database"]);
    assert!(output.status.success());
    let output = api.run(&["--datasource", &url, "drop-database"]);
    assert!(output.status.success());
    assert!(!sqlite_path.exists());
}

#[test_connector(tags(Postgres, Mysql))]
fn test_drop_database(api: TestApi) {
    let connection_string = api.connection_string();
    let output = run(&["--datasource", &connection_string, "drop-database"]);
    assert!(output.status.success(), "{output:#?}");

    let output = run(&["--datasource", &connection_string, "can-connect-to-database"]);
    assert_eq!(output.status.code(), Some(1));

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains(DatabaseDoesNotExist::ERROR_CODE));
}

#[test_connector(tags(Mssql))]
fn test_drop_sqlserver_database(api: TestApi) {
    let mut connection_string: JdbcString = format!("jdbc:{}", api.connection_string()).parse().unwrap();

    connection_string
        .properties_mut()
        .insert(String::from("database"), String::from("NEWDATABASE"));

    let connection_string = connection_string.to_string().replace("jdbc:", "");

    let output = api.run(&["--datasource", &connection_string, "create-database"]);
    assert!(output.status.success());

    let output = api.run(&["--datasource", &connection_string, "drop-database"]);
    assert!(output.status.success());

    let output = api.run(&["--datasource", &connection_string, "can-connect-to-database"]);
    assert_eq!(output.status.code(), Some(1));

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains(DatabaseDoesNotExist::ERROR_CODE));
}

#[test_connector(tags(Postgres))]
fn bad_postgres_url_must_return_a_good_error(api: TestApi) {
    let url = "postgresql://postgres:prisma@localhost:543`/mydb?schema=public";

    let output = api.run(&["--datasource", url, "create-database"]);
    assert_eq!(output.status.code(), Some(1));

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains(r#""error_code":"P1013""#));
    assert!(stderr.contains("invalid port number",));
}

#[test_connector(tags(Postgres))]
fn database_already_exists_must_return_a_proper_error(api: TestApi) {
    let connection_string = api.connection_string();
    let output = api.run(&["--datasource", &connection_string, "create-database"]);
    assert_eq!(output.status.code(), Some(1));

    let (host, port) = {
        let url = Url::parse(&connection_string).unwrap();
        (url.host().unwrap().to_string(), url.port().unwrap())
    };

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains(r#""error_code":"P1009""#));
    assert!(stderr.contains(&format!("Database `database_already_exists_must_return_a_proper_error` already exists on the database server at `{host}:{port}`")));
}

#[test_connector(tags(Postgres))]
fn tls_errors_must_be_mapped_in_the_cli(api: TestApi) {
    let connection_string = api.connection_string();
    let url = format!("{connection_string}&sslmode=require&sslaccept=strict");
    let output = api.run(&["--datasource", &url, "can-connect-to-database"]);

    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains(r#""error_code":"P1011""#));
    assert!(
        stderr.contains("Error opening a TLS connection: error performing TLS handshake: server does not support TLS")
    );
}

#[test_connector(tags(Postgres))]
fn basic_jsonrpc_roundtrip_works_with_no_params(_api: TestApi) {
    let tmpdir = tempfile::tempdir().unwrap();
    let tmpfile = tmpdir.path().join("datamodel");

    let datamodel = r#"
        datasource db {
            provider = "postgres"
            url = env("TEST_DATABASE_URL")
        }
    "#;

    fs::create_dir_all(&tmpdir).unwrap();
    fs::write(&tmpfile, datamodel).unwrap();

    let mut command = Command::new(schema_engine_bin_path());
    command.arg("--datamodels").arg(&tmpfile).env("RUST_LOG", "info");

    with_child_process(command, |process| {
        let stdin = process.stdin.as_mut().unwrap();
        let mut stdout = BufReader::new(process.stdout.as_mut().unwrap());

        for _ in 0..2 {
            writeln!(
                stdin,
                r#"{{ "jsonrpc": "2.0", "method": "getDatabaseVersion", "id": 1 }}"#,
            )
            .unwrap();

            let mut response = String::new();
            stdout.read_line(&mut response).unwrap();

            assert!(response.contains("PostgreSQL") || response.contains("CockroachDB"));
        }
    });
}

#[test_connector(tags(Postgres))]
fn basic_jsonrpc_roundtrip_works_with_params(_api: TestApi) {
    let tmpdir = tempfile::tempdir().unwrap();
    let tmpfile = tmpdir.path().join("datamodel");

    let datamodel = indoc! {r#"
        datasource db {
            provider = "postgres"
            url = env("TEST_DATABASE_URL")
        }
    "#};

    fs::create_dir_all(&tmpdir).unwrap();
    fs::write(&tmpfile, datamodel).unwrap();

    let command = Command::new(schema_engine_bin_path());

    let path = tmpfile.to_str().unwrap();
    let schema_path_params = serde_json::json!({
        "datasource": {
            "tag": "Schema",
            "files": [{ "path": path, "content": datamodel }]
        }
    });

    let connection_string_params = serde_json::json!({
        "datasource": {
            "tag": "ConnectionString",
            "url": std::env::var("TEST_DATABASE_URL").unwrap()
        }
    });

    with_child_process(command, |process| {
        let stdin = process.stdin.as_mut().unwrap();
        let mut stdout = BufReader::new(process.stdout.as_mut().unwrap());

        for _ in 0..2 {
            for params in [&schema_path_params, &connection_string_params] {
                let params_template = serde_json::json!({
                    "jsonrpc": "2.0",
                    "method": "getDatabaseVersion",
                    "params": params,
                    "id": 1
                })
                .to_string();

                writeln!(stdin, "{}", &params_template).unwrap();

                let mut response = String::new();
                stdout.read_line(&mut response).unwrap();

                assert!(response.contains("PostgreSQL") || response.contains("CockroachDB"));
            }
        }
    });
}

#[test]
fn introspect_sqlite_empty_database() {
    let tmpdir = tempfile::tempdir().unwrap();
    let schema = r#"
        datasource db {
            provider = "sqlite"
            url = env("TEST_DATABASE_URL")
        }

    "#;

    fs::File::create(tmpdir.path().join("dev.db")).unwrap();

    let mut command = Command::new(schema_engine_bin_path());
    command.env(
        "TEST_DATABASE_URL",
        format!("file:{}/dev.db", tmpdir.path().to_string_lossy()),
    );

    with_child_process(command, |process| {
        let stdin = process.stdin.as_mut().unwrap();
        let mut stdout = BufReader::new(process.stdout.as_mut().unwrap());

        let msg = serde_json::to_string(&serde_json::json!({
            "jsonrpc": "2.0",
            "method": "introspect",
            "id": 1,
            "params": {
                "schema": { "files": [{ "path": "schema.prisma", "content": schema }] },
                "force": true,
                "compositeTypeDepth": 5,
                "baseDirectoryPath": "./base_directory_path/"
            }
        }))
        .unwrap();
        stdin.write_all(msg.as_bytes()).unwrap();
        stdin.write_all(b"\n").unwrap();

        let mut response = String::new();
        stdout.read_line(&mut response).unwrap();

        assert!(response.starts_with(r#"{"jsonrpc":"2.0","error":{"code":4466,"message":"An error happened. Check the data field for details.","data":{"is_panic":false,"message":"The introspected database was empty.","meta":null,"error_code":"P4001"}},"id":1}"#));
    })
}

#[test]
fn introspect_sqlite_invalid_empty_database() {
    let tmpdir = tempfile::tempdir().unwrap();
    let schema = r#"
        datasource db {
            provider = "sqlite"
            url = env("TEST_DATABASE_URL")
        }

        model something {
            id Int
        }
    "#;

    fs::File::create(tmpdir.path().join("dev.db")).unwrap();

    let mut command = Command::new(schema_engine_bin_path());
    command.env(
        "TEST_DATABASE_URL",
        format!("file:{}/dev.db", tmpdir.path().to_string_lossy()),
    );

    with_child_process(command, |process| {
        let stdin = process.stdin.as_mut().unwrap();
        let mut stdout = BufReader::new(process.stdout.as_mut().unwrap());

        let msg = serde_json::to_string(&serde_json::json!({
            "jsonrpc": "2.0",
            "method": "introspect",
            "id": 1,
            "params": {
                "schema": { "files": [{ "path": "schema.prisma", "content": schema }] },
                "force": true,
                "compositeTypeDepth": -1,
                "baseDirectoryPath": "./base_directory_path/"
            }
        }))
        .unwrap();
        stdin.write_all(msg.as_bytes()).unwrap();
        stdin.write_all(b"\n").unwrap();

        let mut response = String::new();
        stdout.read_line(&mut response).unwrap();

        let expected = expect![[r#"
            {"jsonrpc":"2.0","error":{"code":4466,"message":"An error happened. Check the data field for details.","data":{"is_panic":false,"message":"The introspected database was empty.","meta":null,"error_code":"P4001"}},"id":1}
        "#]];
        expected.assert_eq(&response);
    })
}

#[test_connector(tags(Postgres))]
fn execute_postgres(api: TestApi) {
    /* Drop and create database via `drop-database` and `create-database` */

    let connection_string = api.connection_string();
    let output = api.run(&["--datasource", &connection_string, "drop-database"]);
    assert!(output.status.success(), "{output:#?}");
    let output = api.run(&["--datasource", &connection_string, "create-database"]);
    assert!(output.status.success(), "{output:#?}");

    let tmpdir = tempfile::tempdir().unwrap();
    let schema = r#"
        datasource db {
            provider = "postgres"
            url = env("TEST_DATABASE_URL")
        }
    "#;

    let schema_path = tmpdir.path().join("prisma.schema");
    fs::write(&schema_path, schema).unwrap();

    let command = Command::new(schema_engine_bin_path());

    with_child_process(command, |process| {
        let stdin = process.stdin.as_mut().unwrap();
        let mut stdout = BufReader::new(process.stdout.as_mut().unwrap());

        /* Run `dbExecute` */
        let msg = serde_json::to_string(&serde_json::json!({
            "jsonrpc": "2.0",
            "method": "dbExecute",
            "id": 1,
            "params": {
                "datasourceType": {
                    "tag": "schema",
                    "files": [{ "path": &schema_path, "content": &schema }],
                    "configDir": schema_path.parent().unwrap().to_string_lossy(),
                },
                "script": "SELECT 1;",
            }
        }))
        .unwrap();

        stdin.write_all(msg.as_bytes()).unwrap();
        stdin.write_all(b"\n").unwrap();

        let mut response = String::new();
        stdout.read_line(&mut response).unwrap();

        let expected = expect![[r#"
            {"jsonrpc":"2.0","result":null,"id":1}
        "#]];

        expected.assert_eq(&response);
    });
}

#[test_connector(tags(Postgres), exclude(CockroachDb), preview_features("views"))]
fn introspect_single_postgres_force(api: TestApi) {
    /* Drop and create database via `drop-database` and `create-database` */

    let connection_string = api.connection_string();

    let output = api.run(&["--datasource", &connection_string, "drop-database"]);
    assert!(output.status.success(), "{output:#?}");

    let output = api.run(&["--datasource", &connection_string, "create-database"]);
    assert!(output.status.success(), "{output:#?}");

    let tmpdir = tempfile::tempdir().unwrap();
    let schema = indoc! {r#"
        datasource db {
          provider = "postgres"
          url = env("TEST_DATABASE_URL")
        }

        generator js {
          provider = "prisma-client-js"
          previewFeatures = ["views"]
        }
    "#};

    let schema_path = tmpdir.path().join("schema.prisma");
    fs::write(&schema_path, schema).unwrap();

    let command = Command::new(schema_engine_bin_path());

    with_child_process(command, |process| {
        let stdin = process.stdin.as_mut().unwrap();
        let mut stdout = BufReader::new(process.stdout.as_mut().unwrap());

        /* Create table via `dbExecute` */

        let script = indoc! {r#"
            DROP TABLE IF EXISTS "public"."A";
            DROP VIEW IF EXISTS "public"."B";

            CREATE TABLE "public"."A" (
                id SERIAL PRIMARY KEY,
                data TEXT
            );

            CREATE VIEW "public"."B" AS SELECT 1 AS col;
        "#};

        let msg = serde_json::to_string(&serde_json::json!({
            "jsonrpc": "2.0",
            "method": "dbExecute",
            "id": 1,
            "params": {
                "datasourceType": {
                    "tag": "schema",
                    "files": [{ "path": &schema_path, "content": &schema }],
                    "configDir": schema_path.parent().unwrap().to_string_lossy(),
                },
                "script": script,
            }
        }))
        .unwrap();
        stdin.write_all(msg.as_bytes()).unwrap();
        stdin.write_all(b"\n").unwrap();

        let mut response = String::new();
        stdout.read_line(&mut response).unwrap();

        let expected = expect![[r#"
            {"jsonrpc":"2.0","result":null,"id":1}
        "#]];

        expected.assert_eq(&response);

        /* Introspect via `introspect` */
        let msg = serde_json::to_string(&serde_json::json!({
            "jsonrpc": "2.0",
            "method": "introspect",
            "id": 1,
            "params": {
                "schema": { "files": [{ "path": "./prisma/schema.prisma", "content": &schema }] },
                "force": true,
                "compositeTypeDepth": 5,
                "baseDirectoryPath": "./base_directory_path/"
            }
        }))
        .unwrap();

        stdin.write_all(msg.as_bytes()).unwrap();
        stdin.write_all(b"\n").unwrap();

        let mut response = String::new();
        stdout.read_line(&mut response).unwrap();

        let expected = expect![[r#"
            {"jsonrpc":"2.0","result":{"schema":{"files":[{"content":"generator js {\n  provider        = \"prisma-client-js\"\n  previewFeatures = [\"views\"]\n}\n\ndatasource db {\n  provider = \"postgres\"\n  url      = env(\"TEST_DATABASE_URL\")\n}\n\nmodel A {\n  id   Int     @id @default(autoincrement())\n  data String?\n}\n\n/// The underlying view does not contain a valid unique identifier and can therefore currently not be handled by Prisma Client.\nview B {\n  col Int?\n\n  @@ignore\n}\n","path":"./prisma/schema.prisma"}]},"views":[{"definition":"SELECT\n  1 AS col;","name":"B","schema":"public"}],"warnings":"*** WARNING ***\n\nThe following views were ignored as they do not have a valid unique identifier or id. This is currently not supported by Prisma Client. Please refer to the documentation on defining unique identifiers in views: https://pris.ly/d/view-identifiers\n  - \"B\"\n"},"id":1}
        "#]];

        expected.assert_eq(&response);
    });
}

#[test_connector(tags(Postgres), exclude(CockroachDb), preview_features("views"))]
fn introspect_multi_postgres_force(api: TestApi) {
    /* Drop and create database via `drop-database` and `create-database` */

    let connection_string = api.connection_string();

    let output = api.run(&["--datasource", &connection_string, "drop-database"]);
    assert!(output.status.success(), "{output:#?}");

    let output = api.run(&["--datasource", &connection_string, "create-database"]);
    assert!(output.status.success(), "{output:#?}");

    let (tmpdir, files) = write_multi_file_vec! {
        "a.prisma" => r#"
            datasource db {
                provider = "postgres"
                url = env("TEST_DATABASE_URL")
            }
        "#,
        "b.prisma" => r#"
            model User {
                id Int @id
            }
        "#,
    };

    let files = files
        .into_iter()
        .map(|(schema_path, content)| SchemaContainer {
            path: schema_path,
            content: content.to_string(),
        })
        .collect::<Vec<_>>();

    for file in &files {
        fs::write(&file.path, &file.content).unwrap();
    }

    let command = Command::new(schema_engine_bin_path());

    with_child_process(command, |process| {
        let stdin = process.stdin.as_mut().unwrap();
        let mut stdout = BufReader::new(process.stdout.as_mut().unwrap());

        /* Create table via `dbExecute` */

        let script = indoc! {r#"
            DROP TABLE IF EXISTS "public"."A";
            DROP VIEW IF EXISTS "public"."B";

            CREATE TABLE "public"."A" (
                id SERIAL PRIMARY KEY,
                data TEXT
            );

            CREATE VIEW "public"."B" AS SELECT 1 AS col;
        "#};

        let msg = serde_json::to_string(&serde_json::json!({
            "jsonrpc": "2.0",
            "method": "dbExecute",
            "id": 1,
            "params": {
                "datasourceType": {
                    "tag": "schema",
                    "files": files,
                    "configDir": tmpdir.path().to_string_lossy().to_string(),
                },
                "script": script,
            }
        }))
        .unwrap();
        stdin.write_all(msg.as_bytes()).unwrap();
        stdin.write_all(b"\n").unwrap();

        let mut response = String::new();
        stdout.read_line(&mut response).unwrap();

        let expected = expect![[r#"
            {"jsonrpc":"2.0","result":null,"id":1}
        "#]];

        expected.assert_eq(&response);

        /* Introspect via `introspect` */
        let msg = serde_json::to_string(&serde_json::json!({
            "jsonrpc": "2.0",
            "method": "introspect",
            "id": 1,
            "params": {
                "schema": { "files": files },
                "force": true,
                "compositeTypeDepth": 5,
                "baseDirectoryPath": "./base_directory_path/"
            }
        }))
        .unwrap();

        stdin.write_all(msg.as_bytes()).unwrap();
        stdin.write_all(b"\n").unwrap();

        let mut response = String::new();
        stdout.read_line(&mut response).unwrap();

        let expected = expect![[r#"
            {"jsonrpc":"2.0","result":{"schema":{"files":[{"content":"datasource db {\n  provider = \"postgres\"\n  url      = env(\"TEST_DATABASE_URL\")\n}\n\nmodel A {\n  id   Int     @id @default(autoincrement())\n  data String?\n}\n","path":"./base_directory_path/introspected.prisma"}]},"views":null,"warnings":null},"id":1}
        "#]];

        expected.assert_eq(&response);
    });
}

// TODO: create a basic table before introspecting
#[test]
#[ignore]
fn introspect_e2e() {
    use std::io::{BufRead, BufReader, Write as _};
    let tmpdir = tempfile::tempdir().unwrap();
    let schema = r#"
        datasource db {
            provider = "sqlite"
            url = env("TEST_DATABASE_URL")
        }

    "#;
    fs::File::create(tmpdir.path().join("dev.db")).unwrap();

    let mut command = Command::new(schema_engine_bin_path());

    command.env(
        "TEST_DATABASE_URL",
        format!("file:{}/dev.db", tmpdir.path().to_string_lossy()),
    );

    with_child_process(command, |process| {
        let stdin = process.stdin.as_mut().unwrap();
        let mut stdout = BufReader::new(process.stdout.as_mut().unwrap());

        let msg = serde_json::to_string(&serde_json::json!({
            "jsonrpc": "2.0",
            "method": "introspect",
            "id": 1,
            "params": {
                "schema": schema,
                "force": true,
                "compositeTypeDepth": 5,
                "baseDirectoryPath": "./base_directory_path/",
            }
        }))
        .unwrap();
        stdin.write_all(msg.as_bytes()).unwrap();
        stdin.write_all(b"\n").unwrap();

        let mut response = String::new();
        stdout.read_line(&mut response).unwrap();

        dbg!("response: {:?}", &response);

        assert!(response.starts_with(r#"{"jsonrpc":"2.0","result":{"datamodel":"datasource db {\n  provider = \"sqlite\"\n  url      = env(\"TEST_DATABASE_URL\")\n}\n","warnings":[]},"#));
    });
}

fn to_schema_containers(files: Vec<(String, &str)>) -> Vec<SchemaContainer> {
    files
        .into_iter()
        .map(|(path, content)| SchemaContainer {
            path: path.to_string(),
            content: content.to_string(),
        })
        .collect()
}

fn to_schemas_container(files: Vec<(String, &str)>) -> SchemasContainer {
    SchemasContainer {
        files: to_schema_containers(files),
    }
}

#[test_connector(tags(Postgres))]
fn get_database_version_multi_file(_api: TestApi) {
    let (_, files) = write_multi_file_vec! {
        "a.prisma" => r#"
            datasource db {
                provider = "postgres"
                url = env("TEST_DATABASE_URL")
            }
        "#,
        "b.prisma" => r#"
            model User {
                id Int @id
            }
        "#,
    };

    let command = Command::new(schema_engine_bin_path());

    let schema_path_params = GetDatabaseVersionInput {
        datasource: DatasourceParam::Schema(to_schemas_container(files)),
    };

    let connection_string_params = GetDatabaseVersionInput {
        datasource: DatasourceParam::ConnectionString(UrlContainer {
            url: std::env::var("TEST_DATABASE_URL").unwrap(),
        }),
    };

    with_child_process(command, |process| {
        let stdin = process.stdin.as_mut().unwrap();
        let mut stdout = BufReader::new(process.stdout.as_mut().unwrap());

        for _ in 0..2 {
            for params in [&schema_path_params, &connection_string_params] {
                let params_template = serde_json::json!({
                    "jsonrpc": "2.0",
                    "method": "getDatabaseVersion",
                    "params": params,
                    "id": 1
                })
                .to_string();

                writeln!(stdin, "{}", &params_template).unwrap();

                let mut response = String::new();
                stdout.read_line(&mut response).unwrap();

                assert!(response.contains("PostgreSQL") || response.contains("CockroachDB"));
            }
        }
    });
}
