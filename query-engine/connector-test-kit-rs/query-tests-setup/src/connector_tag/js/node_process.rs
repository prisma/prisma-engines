use super::*;
use once_cell::sync::Lazy;
use serde::de::DeserializeOwned;
use std::{
    io::{self, Write},
    sync::atomic::Ordering,
};
use tokio::sync::{mpsc, oneshot};

pub(crate) struct ExecutorProcess {
    task_handle: mpsc::Sender<ReqImpl>,
    request_id_counter: AtomicU64,
}

fn exit_with_message(status_code: i32, message: &str) -> ! {
    let stdout = std::io::stdout();
    stdout.lock().write_all(message.as_bytes()).unwrap();
    std::process::exit(1)
}

impl ExecutorProcess {
    fn new() -> std::io::Result<(ExecutorProcess, ProcessConfig)> {
        let (sender, receiver) = mpsc::channel::<ReqImpl>(300);
        let (init_sender, init_receiver) = oneshot::channel::<ProcessConfig>();

        std::thread::spawn(|| match start_rpc_thread(receiver, init_sender) {
            Ok(()) => (),
            Err(err) => {
                exit_with_message(1, &err.to_string());
            }
        });

        let process = ExecutorProcess {
            task_handle: sender,
            request_id_counter: Default::default(),
        };
        let config = init_receiver
            .blocking_recv()
            .map_err(|err| io::Error::new(io::ErrorKind::Other, err))?;

        Ok((process, config))
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

pub(super) static NODE_PROCESS: Lazy<(ExecutorProcess, ProcessConfig)> =
    Lazy::new(|| match std::panic::catch_unwind(ExecutorProcess::new) {
        Ok(Ok(process)) => process,
        Ok(Err(err)) => exit_with_message(1, &format!("Failed to start node process. Details: {err}")),
        Err(_) => exit_with_message(1, "Panic while trying to start node process."),
    });

type ReqImpl = (jsonrpc_core::MethodCall, oneshot::Sender<serde_json::value::Value>);

#[derive(Default, Deserialize)]
pub(super) struct ProcessConfig {
    pub(super) datamodel_provider: String,
}

fn start_rpc_thread(
    mut receiver: mpsc::Receiver<ReqImpl>,
    init_sender: oneshot::Sender<ProcessConfig>,
) -> std::io::Result<()> {
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
                            exit_with_message(1, "child node process stdout closed")
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
