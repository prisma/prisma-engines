use quaint::{prelude::Queryable, single::Quaint};
use schema_core::schema_connector::{ConnectorError, ConnectorResult};
use std::{
    future::Future,
    pin::Pin,
    sync::{mpsc, OnceLock},
};
use test_setup::{mysql::mysql_safe_identifier, runtime::run_with_thread_local_runtime as tok};
use url::Url;

pub(crate) async fn mysql_setup(url: String, prisma_schema: &str) -> ConnectorResult<()> {
    mysql_reset(&url).await?;
    let mut connector = sql_schema_connector::SqlSchemaConnector::new_mysql();
    crate::diff_and_apply(prisma_schema, url, &mut connector).await
}

async fn mysql_reset(original_url: &str) -> ConnectorResult<()> {
    let url = Url::parse(original_url).map_err(ConnectorError::url_parse_error)?;
    let db_name = url.path().trim_start_matches('/');
    create_mysql_database(original_url, db_name).await
}

async fn create_mysql_database(database_url: &str, db_name: &str) -> ConnectorResult<()> {
    type Message = Box<dyn (for<'a> FnOnce(&'a Quaint) -> Pin<Box<dyn Future<Output = ()> + 'a>>) + Send>;
    static ADMIN_THREAD_HANDLE: OnceLock<mpsc::SyncSender<Message>> = OnceLock::new();

    let handle = ADMIN_THREAD_HANDLE.get_or_init(|| {
        let mut mysql_db_url: Url = database_url.parse().unwrap();
        let (sender, receiver) = mpsc::sync_channel::<Message>(200);

        debug_assert!(!db_name.is_empty());
        debug_assert!(
            db_name.len() <= 64,
            "db_name should be at most 64 characters, got {:?}",
            db_name.len()
        );

        mysql_db_url.set_path("/mysql");

        std::thread::spawn(move || {
            let conn = tok(Quaint::new(mysql_db_url.as_ref())).unwrap();

            for msg in receiver.iter() {
                tok(msg(&conn))
            }
        });

        sender
    });

    let db_name = mysql_safe_identifier(db_name);

    let drop = format!(
        r#"
        DROP DATABASE IF EXISTS `{db_name}`;
        "#,
    );

    let recreate = format!(
        r#"
        CREATE DATABASE `{db_name}`;
        "#,
    );

    let (wait_sender, wait_recv) = std::sync::mpsc::sync_channel(1);

    handle
        .send(Box::new(move |conn| {
            Box::pin(async move {
                // The two commands have to be run separately on mariadb.
                conn.raw_cmd(&drop).await.unwrap();
                conn.raw_cmd(&recreate).await.unwrap();
                wait_sender.send(()).unwrap();
            })
        }))
        .unwrap();

    wait_recv.recv().unwrap();
    Ok(())
}
