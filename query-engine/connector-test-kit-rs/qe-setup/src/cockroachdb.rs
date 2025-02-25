use std::sync::OnceLock;

use quaint::{connector::PostgresFlavour, prelude::*, single::Quaint};
use schema_core::schema_connector::{ConnectorError, ConnectorResult};
use url::Url;

pub(crate) async fn cockroach_setup(url: String, prisma_schema: &str) -> ConnectorResult<()> {
    let mut parsed_url = Url::parse(&url).map_err(ConnectorError::url_parse_error)?;
    let mut quaint_url = quaint::connector::PostgresNativeUrl::new(parsed_url.clone()).unwrap();
    quaint_url.set_flavour(PostgresFlavour::Cockroach);

    let db_name = quaint_url.dbname();
    let conn = create_admin_conn(&mut parsed_url).await?;

    let query = format!(
        r#"
        DROP DATABASE IF EXISTS "{db_name}";
        CREATE DATABASE "{db_name}";
        "#
    );

    conn.raw_cmd(&query).await.unwrap();

    drop_db_when_thread_exits(parsed_url, db_name);
    let mut connector = sql_schema_connector::SqlSchemaConnector::new_cockroach();
    crate::diff_and_apply(prisma_schema, url, &mut connector).await
}

async fn create_admin_conn(url: &mut Url) -> ConnectorResult<Quaint> {
    url.set_path("/postgres");
    Ok(Quaint::new(url.as_ref()).await.unwrap())
}

fn drop_db_when_thread_exits(admin_url: Url, db_name: &str) {
    use std::{cell::RefCell, sync::mpsc, thread};
    use test_setup::runtime::run_with_thread_local_runtime as tok;

    // === Dramatis Personæ ===

    // DB_DROP_THREAD: A thread that drops databases.
    static DB_DROP_THREAD: OnceLock<mpsc::SyncSender<String>> = OnceLock::new();

    let sender = DB_DROP_THREAD.get_or_init(|| {
        let (sender, receiver) = mpsc::sync_channel::<String>(4096);

        thread::spawn(move || {
            let mut admin_url = admin_url;
            let conn = tok(create_admin_conn(&mut admin_url)).unwrap();

            // Receive new databases to drop.
            for msg in receiver.iter() {
                tok(conn.raw_cmd(&msg)).unwrap();
            }
        });

        sender
    });

    // NOTIFIER: a thread local that notifies DB_DROP_THREAD when dropped.
    struct Notifier(String, mpsc::SyncSender<String>);

    impl Drop for Notifier {
        fn drop(&mut self) {
            let message = std::mem::take(&mut self.0);

            self.1.send(message).unwrap();
        }
    }

    thread_local! {
        static NOTIFIER: RefCell<Option<Notifier>> = const { RefCell::new(None) };
    }

    NOTIFIER.with(move |cell| {
        *cell.borrow_mut() = Some(Notifier(format!("DROP DATABASE \"{db_name}\""), sender.clone()));
    });
}
