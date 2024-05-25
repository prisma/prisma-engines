use super::*;
use once_cell::sync::Lazy;
use serde::de::DeserializeOwned;
use std::{
    error::Error as StdError,
    fmt::Display,
    io::Write as _,
    sync::{atomic::Ordering, Arc},
};
use tokio::sync::{mpsc, oneshot, RwLock};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

pub(crate) struct ExecutorProcess {
    task_handle: mpsc::Sender<ReqImpl>,
    request_id_counter: AtomicU64,
}

fn exit_with_message(status_code: i32, message: &str) -> ! {
    let stdout = std::io::stdout();
    stdout.lock().write_all(message.as_bytes()).unwrap();
    std::process::exit(status_code)
}

impl ExecutorProcess {
    fn spawn() -> ExecutorProcess {
        match std::thread::spawn(ExecutorProcess::new).join() {
            Ok(Ok(process)) => process,
            Ok(Err(err)) => exit_with_message(1, &format!("Failed to start node process. Details: {err}")),
            Err(err) => {
                let err = err.downcast_ref::<String>().map(ToOwned::to_owned).unwrap_or_default();
                exit_with_message(1, &format!("Panic while trying to start node process.\nDetails: {err}"))
            }
        }
    }

    fn new() -> Result<ExecutorProcess> {
        let (sender, receiver) = mpsc::channel::<ReqImpl>(300);

        let handle = std::thread::spawn(|| match start_rpc_thread(receiver) {
            Ok(()) => (),
            Err(err) => {
                exit_with_message(1, &err.to_string());
            }
        });

        std::thread::spawn(move || {
            if let Err(e) = handle.join() {
                exit_with_message(
                    1,
                    &format!(
                        "rpc thread panicked with: {}",
                        e.downcast::<String>().unwrap_or_default()
                    ),
                );
            }
        });

        Ok(ExecutorProcess {
            task_handle: sender,
            request_id_counter: Default::default(),
        })
    }

    /// Convenient façade. Allocates more than necessary, but this is only for testing.
    #[tracing::instrument(skip(self))]
    pub(crate) async fn request<T: DeserializeOwned>(&self, method: &str, params: serde_json::Value) -> Result<T> {
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
        let raw_response = receiver.await??;
        tracing::debug!(%raw_response);
        let response = serde_json::from_value(raw_response)?;
        Ok(response)
    }
}

/// Wraps an ExecutorProcess allowing for restarting it.
///
/// A node process can die for a number of reasons, being one that any `panic!` occurring in Rust
/// asynchronous code are translated to an abort trap by wasm-bindgen, which kills the node process.
#[derive(Clone)]
pub(crate) struct RestartableExecutorProcess {
    process: Arc<RwLock<ExecutorProcess>>,
}

impl RestartableExecutorProcess {
    fn new() -> Self {
        Self {
            process: Arc::new(RwLock::new(ExecutorProcess::spawn())),
        }
    }

    async fn restart(&self) {
        let mut process = self.process.write().await;
        *process = ExecutorProcess::spawn();
    }

    pub(crate) async fn request<T: DeserializeOwned>(&self, method: &str, params: serde_json::Value) -> Result<T> {
        let p = self.process.read().await;
        p.request(method, params).await
    }
}

struct ExecutorProcessDiedError;

impl fmt::Debug for ExecutorProcessDiedError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "The external test executor process died")
    }
}

impl Display for ExecutorProcessDiedError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self, f)
    }
}

impl StdError for ExecutorProcessDiedError {}

struct PendingRequests {
    map: HashMap<jsonrpc_core::Id, oneshot::Sender<Result<serde_json::value::Value>>>,
    last_id: Option<jsonrpc_core::Id>,
}

impl PendingRequests {
    fn new() -> Self {
        Self {
            map: HashMap::new(),
            last_id: None,
        }
    }

    fn insert(&mut self, id: jsonrpc_core::Id, sender: oneshot::Sender<Result<serde_json::value::Value>>) {
        self.map.insert(id.clone(), sender);
        self.last_id = Some(id);
    }

    fn respond(&mut self, id: &jsonrpc_core::Id, response: Result<serde_json::value::Value>) {
        self.map
            .remove(id)
            .expect("no sender for response")
            .send(response)
            .unwrap();
    }

    fn respond_to_last(&mut self, response: Result<serde_json::value::Value>) {
        let last_id = self
            .last_id
            .as_ref()
            .expect("Expected last response to exist")
            .to_owned();
        self.respond(&last_id, response);
    }
}

pub(super) static EXTERNAL_PROCESS: Lazy<RestartableExecutorProcess> = Lazy::new(RestartableExecutorProcess::new);

type ReqImpl = (
    jsonrpc_core::MethodCall,
    oneshot::Sender<Result<serde_json::value::Value>>,
);

fn start_rpc_thread(mut receiver: mpsc::Receiver<ReqImpl>) -> Result<()> {
    use std::process::Stdio;
    use tokio::process::Command;

    let path = crate::CONFIG
        .external_test_executor_path()
        .unwrap_or_else(|| exit_with_message(1, "start_rpc_thread() error: external test executor is not set"));

    tokio::runtime::Builder::new_current_thread()
        .enable_io()
        .build()
        .unwrap()
        .block_on(async move {
            let environment = CONFIG.for_external_executor();
            let process = match Command::new(&path)
                .envs(environment)
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::inherit())
                .spawn()
            {
                Ok(process) => process,
                Err(err) => exit_with_message(1, &format!("Failed to spawn the executor process: `{path}`. Details: {err}\n")),
            };

            let mut stdout = BufReader::new(process.stdout.unwrap()).lines();
            let mut stdin = process.stdin.unwrap();
            let mut pending_requests = PendingRequests::new();

            loop {
                tokio::select! {
                    line = stdout.next_line() => {
                        match line {
                            // Two error modes in here: the external process can response with
                            // something that is not a jsonrpc response (basically any normal logging
                            // output), or it can respond with a jsonrpc response that represents a
                            // failure.
                            Ok(Some(line)) => // new response
                            {
                                match serde_json::from_str::<jsonrpc_core::Output>(&line) {
                                    Ok(ref response) => {
                                        let res: Result<serde_json::value::Value> = match response {
                                            jsonrpc_core::Output::Success(success) => {
                                                // The other end may be dropped if the whole
                                                // request future was dropped and not polled to
                                                // completion, so we ignore send errors here.
                                                Ok(success.result.clone())
                                            }
                                            jsonrpc_core::Output::Failure(err) => {
                                                tracing::error!("error response from jsonrpc: {err:?}");
                                                Err(Box::new(err.error.clone()))
                                            }
                                        };
                                        pending_requests.respond(response.id(), res)
                                    }
                                    Err(err) => {
                                        tracing::error!(%err, "error when decoding response from child node process. Response was: `{}`", &line);
                                        continue
                                    }
                                };
                            }
                            Ok(None) => // end of the stream
                            {
                                tracing::error!("Error when reading from child node process. Process might have exited. Restarting...");

                                pending_requests.respond_to_last(Err(Box::new(ExecutorProcessDiedError)));
                                EXTERNAL_PROCESS.restart().await;
                                break;
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
                                exit_with_message(1, "The json-rpc client channel was closed");
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
