use super::CliError;
use clap::ArgMatches;
use quaint::{prelude::*, single::Quaint};

fn run_sync(matches: &ArgMatches<'_>, datasource: &str) -> Result<String, CliError> {
    test_setup::runtime::run_with_tokio(super::run(matches, datasource))
}

async fn run(args: &[&str], datasource: &str) -> Result<String, CliError> {
    let mut complete_args = vec!["me", "cli", "--datasource", datasource];
    complete_args.extend(args);
    let matches = super::clap_app().get_matches_from(complete_args);
    super::run(&matches.subcommand_matches("cli").unwrap(), datasource).await
}

fn with_cli<F>(matches: Vec<&str>, f: F) -> Result<(), Box<dyn std::any::Any + Send + 'static>>
where
    F: FnOnce(&clap::ArgMatches) -> () + std::panic::UnwindSafe,
{
    let matches = clap::App::new("cli")
        .arg(
            clap::Arg::with_name("can_connect_to_database")
                .long("can_connect_to_database")
                .takes_value(false)
                .required(false),
        )
        .arg(
            clap::Arg::with_name("create_database")
                .long("create_database")
                .help("Create an empty database defined in the configuration string.")
                .takes_value(false)
                .required(false),
        )
        .get_matches_from(matches);

    std::panic::catch_unwind(|| f(&matches))
}

fn postgres_url(db: Option<&str>) -> String {
    postgres_url_with_scheme(db, "postgresql")
}

fn postgres_url_with_scheme(db: Option<&str>, scheme: &str) -> String {
    let original_url = test_setup::postgres_10_url(db.unwrap_or("postgres"));
    let mut parsed: url::Url = original_url.parse().unwrap();
    parsed.set_scheme(scheme).unwrap();
    parsed.to_string()
}

fn mysql_url(db: Option<&str>) -> String {
    test_setup::mysql_url(db.unwrap_or(""))
}

#[test]
fn test_with_missing_command() {
    with_cli(vec!["cli"], |matches| {
        assert_eq!(
            "No command defined",
            &run_sync(&matches, &mysql_url(None)).unwrap_err().to_string()
        );
    })
    .unwrap();
}

#[test]
fn test_connecting_with_a_working_mysql_connection_string() {
    with_cli(vec!["cli", "--can_connect_to_database"], |matches| {
        assert_eq!(
            String::from("Connection successful"),
            run_sync(&matches, &mysql_url(None)).unwrap()
        )
    })
    .unwrap();
}

#[test]
fn test_connecting_with_a_non_working_mysql_connection_string() {
    let dm = mysql_url(Some("this_does_not_exist"));

    with_cli(vec!["cli", "--can_connect_to_database"], |matches| {
        assert_eq!("P1003", run_sync(&matches, &dm).unwrap_err().error_code().unwrap());
    })
    .unwrap();
}

#[test]
fn test_connecting_with_a_working_psql_connection_string() {
    with_cli(vec!["cli", "--can_connect_to_database"], |matches| {
        assert_eq!(
            String::from("Connection successful"),
            run_sync(&matches, &postgres_url(None)).unwrap()
        );
    })
    .unwrap();
}

#[test]
fn test_connecting_with_a_working_psql_connection_string_with_postgres_scheme() {
    with_cli(vec!["cli", "--can_connect_to_database"], |matches| {
        assert_eq!(
            String::from("Connection successful"),
            run_sync(&matches, &postgres_url_with_scheme(None, "postgres")).unwrap()
        );
    })
    .unwrap();
}

#[test]
fn test_connecting_with_a_non_working_psql_connection_string() {
    let dm = postgres_url(Some("this_does_not_exist"));

    with_cli(vec!["cli", "--can_connect_to_database"], |matches| {
        assert_eq!("P1003", run_sync(&matches, &dm).unwrap_err().error_code().unwrap());
    })
    .unwrap();
}

#[tokio::test]
async fn test_create_mysql_database() {
    let url = mysql_url(Some("this_should_exist"));

    let res = run(&["--create_database"], &url).await;

    assert_eq!(
        "Database 'this_should_exist' created successfully.",
        res.as_ref().unwrap()
    );

    if let Ok(_) = res {
        let res = run(&["--can_connect_to_database"], &url).await;
        assert_eq!("Connection successful", res.as_ref().unwrap());

        {
            let uri = mysql_url(None);
            let conn = Quaint::new(&uri).await.unwrap();

            conn.execute_raw("DROP DATABASE `this_should_exist`", &[])
                .await
                .unwrap();
        }

        res.unwrap();
    } else {
        res.unwrap();
    }
}

#[tokio::test]
async fn test_create_psql_database() {
    let db_name = "this_should_exist";

    let _drop_database: () = {
        let url = postgres_url(None);

        let conn = Quaint::new(&url).await.unwrap();

        conn.execute_raw("DROP DATABASE IF EXISTS \"this_should_exist\"", &[])
            .await
            .unwrap();
    };

    let url = postgres_url(Some(db_name));

    let res = run(&["--create_database"], &url).await;

    assert_eq!(
        "Database 'this_should_exist' created successfully.",
        res.as_ref().unwrap()
    );

    let res = run(&["--can_connect_to_database"], &url).await;
    assert_eq!("Connection successful", res.as_ref().unwrap());

    res.unwrap();
}

#[tokio::test]
async fn test_create_sqlite_database() {
    let base_dir = tempfile::tempdir().unwrap();

    let sqlite_path = base_dir
        .path()
        .join("doesntexist/either")
        .join("test_create_sqlite_database.db");

    assert!(!sqlite_path.exists());

    let url = format!("file:{}", sqlite_path.to_string_lossy());

    let res = run(&["--create_database"], &url).await;
    assert_eq!("", res.as_ref().unwrap());

    assert!(sqlite_path.exists());
}

#[test]
fn test_fetch_db_name() {
    let url: url::Url = "postgresql://postgres:prisma@127.0.0.1:5432/pgres?schema=test_schema"
        .parse()
        .unwrap();
    let db_name = super::fetch_db_name(&url, "postgres");
    assert_eq!(db_name, "pgres");
}

#[test]
fn test_fetch_db_name_with_postgres_scheme() {
    let url: url::Url = "postgres://postgres:prisma@127.0.0.1:5432/pgres?schema=test_schema"
        .parse()
        .unwrap();
    let db_name = super::fetch_db_name(&url, "postgres");
    assert_eq!(db_name, "pgres");
}
