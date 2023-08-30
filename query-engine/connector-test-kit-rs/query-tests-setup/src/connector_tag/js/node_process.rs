use super::*;
use once_cell::sync::Lazy;
use query_core::{
    executor::TransactionManager, protocol::EngineProtocol, response_ir::ResponseData, schema::QuerySchemaRef,
    BatchDocumentTransaction, Connector, Operation, QueryExecutor, TransactionOptions, TxId,
};
use serde::de::DeserializeOwned;
use std::{io::Write, sync::atomic::Ordering};
use tokio::sync::{mpsc, oneshot};

struct ExecutorThread {}

impl ExecutorProcess {
    fn new() -> std::io::Result<ExecutorProcess> {
        let (sender, receiver) = mpsc::channel::<ReqImpl>(300);

        std::thread::spawn(|| match start_rpc_thread(receiver) {
            Ok(()) => (),
            Err(err) => {
                tracing::error!("{err}"); // TODO print to stdout
                std::process::exit(1);
            }
        });

        Ok(ExecutorProcess {
            task_handle: sender,
            request_id_counter: Default::default(),
            config: panic!(),
        })
    }

    /// Convenient façade. Allocates more than necessary, but this is only for testing.
    pub(crate) async fn request<T: DeserializeOwned>(
        &self,
        method: &str,
        params: serde_json::Value,
    ) -> Result<T, Box<dyn std::error::Error>> {
        let (sender, receiver) = oneshot::channel();
        let params = if let serde_json::Value::Object(params) = params {
            params
        } else {
            panic!("params aren't an object")
        };
        let method_call = jsonrpc_core::MethodCall {
            jsonrpc: Some(jsonrpc_core::Version::V2),
            method: method.to_owned(),
            params: jsonrpc_core::Params::Map(params),
            id: jsonrpc_core::Id::Num(self.request_id_counter.fetch_add(1, Ordering::Relaxed)),
        };

        self.task_handle.send((method_call, sender)).await?;
        let raw_response = receiver.await?;
        let response = serde_json::from_value(raw_response)?;
        Ok(response)
    }
}

pub(crate) static NODE_PROCESS: Lazy<ExecutorProcess> =
    Lazy::new(|| match std::panic::catch_unwind(ExecutorProcess::new) {
        Ok(Ok(process)) => process,
        Ok(Err(err)) => {
            let mut stdout = std::io::stdout();
            writeln!(stdout, "Failed to start node process. Details: {err}");
            std::process::exit(1);
        }
        Err(_) => {
            let mut stdout = std::io::stdout();
            stdout.write_all(b"Panic while trying to start node process.").unwrap();
            std::process::exit(1);
        }
    });

#[async_trait::async_trait]
impl TransactionManager for ExecutorProcess {
    async fn start_tx(
        &self,
        query_schema: QuerySchemaRef,
        engine_protocol: EngineProtocol,
        opts: TransactionOptions,
    ) -> query_core::Result<TxId> {
        todo!()
    }

    async fn commit_tx(&self, tx_id: TxId) -> Result<(), query_core::CoreError> {
        todo!()
    }

    async fn rollback_tx(&self, tx_id: TxId) -> Result<(), query_core::CoreError> {
        todo!()
    }
}

#[async_trait::async_trait]
impl QueryExecutor for ExecutorProcess {
    async fn execute(
        &self,
        tx_id: Option<TxId>,
        operation: Operation,
        query_schema: QuerySchemaRef,
        trace_id: Option<String>,
        engine_protocol: EngineProtocol,
    ) -> query_core::Result<ResponseData> {
        todo!()
    }

    async fn execute_all(
        &self,
        tx_id: Option<TxId>,
        operations: Vec<Operation>,
        transaction: Option<BatchDocumentTransaction>,
        query_schema: QuerySchemaRef,
        trace_id: Option<String>,
        engine_protocol: EngineProtocol,
    ) -> query_core::Result<Vec<query_core::Result<ResponseData>>> {
        todo!()
    }

    fn primary_connector(&self) -> &(dyn Connector + Send + Sync) {
        todo!()
    }
}

type ReqImpl = (jsonrpc_core::MethodCall, oneshot::Sender<serde_json::value::Value>);

#[derive(Default, Deserialize)]
struct ProcessConfig {
    datamodel_provider: String,
}

pub(crate) struct ExecutorProcess {
    task_handle: mpsc::Sender<ReqImpl>,
    request_id_counter: AtomicU64,
    config: ProcessConfig,
}

fn start_rpc_thread(mut receiver: mpsc::Receiver<ReqImpl>) -> std::io::Result<()> {
    use std::process::Stdio;
    use tokio::process::Command;

    let env_var =
        std::env::var("NODE_TEST_ADAPTER").map_err(|err| std::io::Error::new(std::io::ErrorKind::Other, err))?;
    let process = Command::new(env_var)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()?;

    tokio::spawn(async move {
        let mut stdout = BufReader::new(process.stdout.unwrap()).lines();
        let mut stdin = process.stdin.unwrap();
        let mut pending_requests: HashMap<jsonrpc_core::Id, oneshot::Sender<serde_json::value::Value>> = HashMap::new();

        loop {
            tokio::select! {
                line = stdout.next_line() => {
                    match line {
                        Ok(Some(line)) => // new response
                        {
                            let response: jsonrpc_core::Output = match serde_json::from_str(&line) {
                                Ok(response) => response,
                                Err(err) => // log it
                                {
                                    tracing::error!(%err, "Error when decoding response from child node process");
                                    continue
                                }
                            };

                            let sender= pending_requests.remove(response.id()).unwrap();
                            match response {
                                jsonrpc_core::Output::Success(success) => {
                                    sender.send(success.result).unwrap();
                                }
                                jsonrpc_core::Output::Failure(err) => {
                                    panic!("error response from jsonrpc: {err:?}")
                                }
                            }

                        }
                        Ok(None) => // end of the stream
                        {
                            tracing::warn!("child node process stdout closed")
                        }
                        Err(err) => // log it
                        {
                            tracing::error!(%err, "Error when reading from child node process");
                        }
                    }
                }
                request = receiver.recv() => {
                    match request {
                        None => // channel closed
                        {
                            tracing::error!("The json-rpc client channel was closed");
                        }
                        Some((request, response_sender)) => {
                            pending_requests.insert(request.id.clone(), response_sender);
                            let mut req = serde_json::to_vec(&request).unwrap();
                            req.push(b'\n');
                            stdin.write_all(&req).await.unwrap();
                        }
                    }
                }
            }
        }
    });

    Ok(())
}
