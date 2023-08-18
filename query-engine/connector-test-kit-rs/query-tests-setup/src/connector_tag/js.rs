use super::*;
use once_cell::sync::Lazy;
use query_core::{executor::TransactionManager, QueryExecutor};
use std::{collections::HashMap, io::Write};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    sync::{mpsc, oneshot},
};

#[derive(Debug, Default, Clone, PartialEq)]
pub struct NodeDrivers;

impl ConnectorTagInterface for NodeDrivers {
    fn datamodel_provider(&self) -> &'static str {
        todo!()
    }

    fn datamodel_renderer(&self) -> Box<dyn DatamodelRenderer> {
        todo!()
    }

    fn connection_string(
        &self,
        test_database: &str,
        is_ci: bool,
        is_multi_schema: bool,
        isolation_level: Option<&'static str>,
    ) -> String {
        todo!()
    }

    fn capabilities(&self) -> ConnectorCapabilities {
        todo!()
    }

    fn as_parse_pair(&self) -> (String, Option<String>) {
        todo!()
    }

    fn is_versioned(&self) -> bool {
        todo!()
    }
}

type ReqImpl = (jsonrpc_core::MethodCall, oneshot::Sender<jsonrpc_core::Response>);

struct NodeProcess {
    task_handle: mpsc::Sender<ReqImpl>,
    request_id_counter: u64,
}

impl NodeProcess {
    fn new() -> std::io::Result<NodeProcess> {
        use std::process::Stdio;
        use tokio::process::Command;

        let env_var =
            std::env::var("NODE_TEST_ADAPTER").map_err(|err| std::io::Error::new(std::io::ErrorKind::Other, err))?;
        let process = Command::new(env_var)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()?;

        let (sender, receiver) = mpsc::channel::<ReqImpl>(300);

        tokio::spawn(async move {
            let stdout = BufReader::new(process.stdout.unwrap()).lines();
            let stdin = process.stdin.unwrap();
            let pending_requests = HashMap::new();

            loop {
                tokio::select! {
                    line = stdout.next_line() => {
                        match line {
                            Ok(Some(line)) => // new response
                            {
                                let response = match serde_json::from_str(&line) {
                                    Ok(response) => response,
                                    Err(err) => // log it
                                    {
                                        tracing::error!(%err, "Error when decoding response from child node process");
                                        continue
                                    }
                                };

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
                                pending_requests.insert(request.id, response_sender);
                                let mut req = serde_json::to_vec(&request).unwrap();
                                req.push(b'\n');
                                stdin.write_all(&req).await.unwrap();
                            }
                        }
                    }
                }
            }
        });

        Ok(NodeProcess {
            task_handle: sender,
            request_id_counter: 0,
        })
    }

    /// Convenient façade. Allocates more than necessary, but this is only for testing.
    async fn request(
        &mut self,
        method: &str,
        params: serde_json::Value,
    ) -> Result<jsonrpc_core::Response, Box<dyn std::error::Error>> {
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
            id: jsonrpc_core::Id::Num(self.request_id_counter),
        };

        self.request_id_counter += 1;
        self.task_handle.send((method_call, sender)).await?;
        Ok(receiver.await?)
    }
}

static NODE_PROCESS: Lazy<NodeProcess> = Lazy::new(|| match std::panic::catch_unwind(NodeProcess::new) {
    Ok(Ok(process)) => process,
    Ok(Err(err)) => {
        let stdout = std::io::stdout();
        writeln!(stdout, "Failed to start node process. Details: {err}");
        std::process::exit(1);
    }
    Err(err) => {
        let stdout = std::io::stdout();
        stdout.write_all(b"Panic while trying to start node process.").unwrap();
        std::process::exit(1);
    }
});

impl TransactionManager for NodeProcess {}

impl QueryExecutor for NodeProcess {}
