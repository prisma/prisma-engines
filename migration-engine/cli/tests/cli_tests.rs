use connection_string::JdbcString;
use std::process::{Command, Output};
use test_macros::test_connector;
use test_setup::{runtime::run_with_thread_local_runtime as tok, BitFlags, Tags, TestApiArgs};
use url::Url;
use user_facing_errors::{common::DatabaseDoesNotExist, UserFacingError};

fn migration_engine_bin_path() -> &'static str {
    env!("CARGO_BIN_EXE_migration-engine")
}

fn run(args: &[&str]) -> Output {
    Command::new(migration_engine_bin_path())
        .arg("cli")
        .args(args)
        .env("RUST_LOG", "INFO")
        .output()
        .unwrap()
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

#[test_connector(tags(Mysql))]
fn test_connecting_with_a_working_mysql_connection_string(api: TestApi) {
    let connection_string = api.connection_string();
    let output = api.run(&["--datasource", &connection_string, "can-connect-to-database"]);

    assert!(output.status.success(), "{:?}", output);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Connection successful"), "{:?}", stderr);
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

    assert!(output.status.success(), "{:?}", output);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Connection successful"), "{:?}", stderr);
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

    assert!(output.status.success(), "{:?}", output);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Connection successful"), "{:?}", stderr);
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

    assert!(output.status.success(), "{:?}", output);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Connection successful"), "{:?}", stderr);
}

#[test_connector(tags(Postgres, Mysql))]
fn test_create_database(api: TestApi) {
    let connection_string = api.connection_string();
    let output = api.run(&["--datasource", &connection_string, "drop-database"]);
    assert!(output.status.success(), "{:#?}", output);

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
    assert!(output.status.success(), "{:?}", stderr);
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
    assert!(output.status.success(), "{:#?}", output);

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
    assert!(stderr.contains(&format!("Database `database_already_exists_must_return_a_proper_error` already exists on the database server at `{host}:{port}`", host = host, port = port)));
}

#[test_connector(tags(Postgres))]
fn tls_errors_must_be_mapped_in_the_cli(api: TestApi) {
    let connection_string = api.connection_string();
    let url = format!("{}&sslmode=require&sslaccept=strict", connection_string);
    let output = api.run(&["--datasource", &url, "can-connect-to-database"]);

    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains(r#""error_code":"P1011""#));
    assert!(
        stderr.contains("Error opening a TLS connection: error performing TLS handshake: server does not support TLS")
    );
}

#[test_connector(tags(Postgres))]
fn basic_jsonrpc_roundtrip_works(_api: TestApi) {
    use std::io::{BufRead, BufReader, Write as _};
    let tmpdir = tempfile::tempdir().unwrap();
    let tmpfile = tmpdir.path().join("datamodel");
    let datamodel = r#"
        datasource db {
            provider = "postgres"
            url = env("TEST_DATABASE_URL")
        }
    "#;
    std::fs::create_dir_all(&tmpdir).unwrap();
    std::fs::write(&tmpfile, datamodel).unwrap();
    let mut process = Command::new(migration_engine_bin_path())
        .arg("--datamodel")
        .arg(&tmpfile)
        .env("RUST_LOG", "INFO")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()
        .unwrap();
    let stdin = process.stdin.as_mut().unwrap();
    let mut stdout = BufReader::new(process.stdout.as_mut().unwrap());

    for _ in 0..2 {
        writeln!(
            stdin,
            r#"{{ "jsonrpc": "2.0", "method": "getDatabaseVersion", "params": {{ }}, "id": 1 }}"#,
        )
        .unwrap();

        let mut response = String::new();
        stdout.read_line(&mut response).unwrap();

        assert!(response.contains("PostgreSQL") || response.contains("CockroachDB"));
    }
}

#[test]
fn introspect_e2e() {
    use std::io::{BufRead, BufReader, Write as _};
    let tmpdir = tempfile::tempdir().unwrap();
    let schema = r#"
        datasource db {
            provider = "sqlite"
            url = env("TEST_DATABASE_URL")
        }

    "#;
    std::fs::File::create(tmpdir.path().join("dev.db")).unwrap();
    let mut process = Command::new(migration_engine_bin_path())
        .env(
            "TEST_DATABASE_URL",
            format!("file:{}/dev.db", tmpdir.path().to_string_lossy()),
        )
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()
        .unwrap();
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
        }
    }))
    .unwrap();
    stdin.write_all(msg.as_bytes()).unwrap();
    stdin.write_all(b"\n").unwrap();

    let mut response = String::new();
    stdout.read_line(&mut response).unwrap();

    assert!(response.starts_with(r##"{"jsonrpc":"2.0","result":{"datamodel":"datasource db {\n  provider = \"sqlite\"\n  url      = env(\"TEST_DATABASE_URL\")\n}\n","version":"NonPrisma","warnings":[]},"##));
}
