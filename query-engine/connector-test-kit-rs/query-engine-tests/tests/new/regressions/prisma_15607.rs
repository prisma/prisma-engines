//! Testing deadlocks on SQL Server deadlocks and deadlock recovery.
//! Due to certain queries hanging forever until the second
//! transaction progresses, the test uses separate engines inside
//! actors to allow test to continue even if one query is blocking.

use indoc::indoc;
use query_engine_tests::{
    query_core::TxId, render_test_datamodel, setup_metrics, test_tracing_subscriber, LogEmit, QueryResult, Runner,
    TestError, TestLogCapture, TestResult, WithSubscriber, CONFIG, ENV_LOG_LEVEL,
};
use std::future::Future;
use tokio::sync::mpsc;

const SCHEMA: &str = indoc! {r#"
    model Country {
      id         Int    @id
      name       String
      cities     City[]
    }

    model City {
      id         Int    @id
      country_id Int
      name       String
      country    Country @relation(fields: [country_id], references: [id])
    }
"#};

struct Actor {
    query_sender: mpsc::Sender<Message>,
    response_receiver: mpsc::Receiver<Response>,
}

#[derive(Debug, Clone)]
enum Message {
    Query(&'static str),
    BeginTransaction,
    RollbackTransaction(TxId),
    SetActiveTx(TxId),
}

#[derive(Debug)]
enum Response {
    Query(TestResult<QueryResult>),
    Tx(TestResult<TxId>),
    Rollback(Result<(), user_facing_errors::Error>),
}

impl Actor {
    /// Spawns a new query engine to the runtime.
    pub async fn spawn() -> TestResult<Self> {
        let (log_capture, log_tx) = TestLogCapture::new();
        async fn with_logs<T>(fut: impl Future<Output = T>, log_tx: LogEmit) -> T {
            fut.with_subscriber(test_tracing_subscriber(
                ENV_LOG_LEVEL.to_string(),
                setup_metrics(),
                log_tx,
            ))
            .await
        }

        let (query_sender, mut query_receiver) = mpsc::channel(100);
        let (response_sender, response_receiver) = mpsc::channel(100);
        let (tag, version) = query_tests_setup::CONFIG.test_connector()?;

        let datamodel = render_test_datamodel(
            "sql_server_deadlocks_test",
            SCHEMA.to_owned(),
            &[],
            None,
            &[],
            Some("READ COMMITTED"),
        );

        let mut runner = Runner::load(datamodel, &[], version, tag, setup_metrics(), log_capture).await?;

        tokio::spawn(async move {
            while let Some(message) = query_receiver.recv().await {
                match message {
                    Message::Query(query) => {
                        let result = with_logs(runner.query(query), log_tx.clone()).await;
                        response_sender.send(Response::Query(result)).await.unwrap();
                    }
                    Message::BeginTransaction => {
                        let response = with_logs(runner.start_tx(10000, 10000, None, None), log_tx.clone()).await;
                        response_sender.send(Response::Tx(response)).await.unwrap();
                    }
                    Message::RollbackTransaction(tx_id) => {
                        let response = with_logs(runner.rollback_tx(tx_id), log_tx.clone()).await?;
                        response_sender.send(Response::Rollback(response)).await.unwrap();
                    }
                    Message::SetActiveTx(tx_id) => {
                        runner.set_active_tx(tx_id);
                    }
                }
            }

            Result::<(), TestError>::Ok(())
        });

        Ok(Self {
            query_sender,
            response_receiver,
        })
    }

    /// Starts a transaction.
    pub async fn begin_tx(&mut self) -> TestResult<TxId> {
        self.query_sender.send(Message::BeginTransaction).await.unwrap();

        match self.response_receiver.recv().await.unwrap() {
            Response::Tx(res) => res,
            Response::Query(_) => Err(TestError::ParseError(
                "Got query response, expected a transaction response".into(),
            )),
            Response::Rollback(_) => Err(TestError::ParseError(
                "Got rollback response, expected a transaction response".into(),
            )),
        }
    }

    /// Rollback the given transaction.
    pub async fn rollback(&mut self, tx_id: TxId) -> TestResult<()> {
        self.query_sender
            .send(Message::RollbackTransaction(tx_id))
            .await
            .unwrap();

        match self.response_receiver.recv().await.unwrap() {
            Response::Rollback(res) => res.map_err(|e| TestError::InteractiveTransactionError(e.message().into())),
            Response::Query(_) => Err(TestError::ParseError(
                "Got query response, expected a rollback response".into(),
            )),
            Response::Tx(_) => Err(TestError::ParseError(
                "Got transaction response, expected a rollback response".into(),
            )),
        }
    }

    /// Sets the given transaction to be active.
    pub async fn set_active_tx_id(&mut self, tx_id: TxId) {
        self.query_sender.send(Message::SetActiveTx(tx_id)).await.unwrap();
    }

    /// Send a query to be executed in the engine. Response must be
    /// fetched in a subsequent call using `recv_query_response`.
    pub async fn send_query(&mut self, query: &'static str) {
        self.query_sender.send(Message::Query(query)).await.unwrap();
    }

    /// Returns the last query response.
    pub async fn recv_query_response(&mut self) -> TestResult<QueryResult> {
        match self.response_receiver.recv().await.unwrap() {
            Response::Query(res) => Ok(res.unwrap()),
            Response::Tx(_) => Err(TestError::ParseError(
                "Got transaction response, expected a query response".into(),
            )),
            Response::Rollback(_) => Err(TestError::ParseError(
                "Got rollback response, expected a query response".into(),
            )),
        }
    }

    /// A helper to run a query and return its response. The query
    /// must be successful.
    pub async fn run_query(&mut self, query: &'static str) -> TestResult<QueryResult> {
        self.send_query(query).await;

        let res = self.recv_query_response().await?;
        res.assert_success();

        Ok(res)
    }
}

#[tokio::test]
async fn sqlserver_can_recover_from_deadlocks() -> TestResult<()> {
    if CONFIG.connector() != "sqlserver" {
        return Ok(());
    }

    let (mut conn1, mut conn2) = (Actor::spawn().await?, Actor::spawn().await?);
    let (tx1, tx2) = (conn1.begin_tx().await?, conn2.begin_tx().await?);

    conn1.set_active_tx_id(tx1.clone()).await;
    conn2.set_active_tx_id(tx2.clone()).await;

    // Queries until the next comment will be successful.
    conn1
        .run_query(r#"mutation { createOneCountry(data: { id: 1, name: "USA" }) { id } }"#)
        .await?;

    conn2
        .run_query(r#"mutation { createOneCountry(data: { id: 2, name: "Finland" }) { id } }"#)
        .await?;

    conn1
        .run_query(r#"query { findUniqueCountry(where: { id: 1 }) { id } }"#)
        .await?;

    conn2
        .run_query(r#"query { findUniqueCountry(where: { id: 2 }) { id } }"#)
        .await?;

    conn1
        .run_query(
            r#"mutation { createOneCity(data: { id: 1, name: "Oakland", country: { connect: { id: 1 } } }) { id } }"#,
        )
        .await?;

    conn2
        .run_query(
            r#"mutation { createOneCity(data: { id: 2, name: "Tampere", country: { connect: { id: 2 } } }) { id } }"#,
        )
        .await?;

    // This will block until the second transaction causes a deadlock.
    conn1
        .send_query(r#"query { findManyCity(where: { country_id: 1 }) { id } }"#)
        .await;

    // Query causes a deadlock.
    conn2
        .send_query(r#"query { findManyCity(where: { country_id: 2 }) { id } }"#)
        .await;

    // Either one of these can be in deadlock, the other being
    // successful.
    let res1 = conn1.recv_query_response().await?;
    let res2 = conn2.recv_query_response().await?;

    if res1.failed() {
        res1.assert_failure(
            2034,
            Some("Transaction failed due to a write conflict or a deadlock.".to_string()),
        );

        // Rollback the successful transaction, so the failed one can continue.
        conn2.rollback(tx2.clone()).await?;

        // The deadlocked query triggers an automatic rollback. The
        // connection must be usable at this point.
        conn1.run_query(r#"query { findManyCity(where: {}) { id } }"#).await?;
    } else if res2.failed() {
        res2.assert_failure(
            2034,
            Some("Transaction failed due to a write conflict or a deadlock".to_string()),
        );

        // Rollback the successful transaction, so the failed one can continue.
        conn1.rollback(tx1.clone()).await?;

        // The deadlocked query triggers an automatic rollback. The
        // connection must be usable at this point.
        conn2.run_query(r#"query { findManyCity(where: {}) { id } }"#).await?;
    } else {
        panic!("Expected one of the queries to fail.");
    }

    Ok(())
}
