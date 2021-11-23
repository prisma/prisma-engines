use jsonrpc_core::{IoHandler, MethodCall, Request};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
};
use tokio::{
    io::{AsyncBufReadExt, AsyncRead, AsyncWrite, AsyncWriteExt},
    sync::{mpsc, oneshot},
};

#[derive(Debug)]
pub struct Client(Arc<ClientInner>, AtomicU64);

impl Client {
    pub fn new() -> (Client, ClientAdapter) {
        let (request_sender, request_receiver) = mpsc::channel(30);
        let client = Client(Arc::new(ClientInner { request_sender }), Default::default());
        let adapter = ClientAdapter { request_receiver };

        (client, adapter)
    }

    pub async fn call<Req, Res>(&self, method: String, params: Req) -> jsonrpc_core::Result<Res>
    where
        Req: serde::Serialize,
        Res: serde::de::DeserializeOwned,
    {
        let id = self.1.fetch_add(1, Ordering::Relaxed);
        let json_params = serde_json::to_value(params).map_err(|_err| jsonrpc_core::Error::invalid_request())?;
        let params = match json_params {
            jsonrpc_core::Value::Array(arr) => jsonrpc_core::Params::Array(arr),
            jsonrpc_core::Value::Object(obj) => jsonrpc_core::Params::Map(obj),
            _ => return Err(jsonrpc_core::Error::invalid_request()),
        };
        let request = jsonrpc_core::MethodCall {
            jsonrpc: Some(jsonrpc_core::Version::V2),
            method,
            params,
            id: jsonrpc_core::Id::Num(id),
        };
        let (response_sender, response_receiver) = oneshot::channel();
        self.0.request_sender.send((request, response_sender)).await.unwrap();

        let response = response_receiver.await.unwrap();

        match response {
            jsonrpc_core::Output::Success(res) => Ok(serde_json::from_value(res.result).unwrap()),
            jsonrpc_core::Output::Failure(res) => Err(res.error),
        }
    }
}

// The other side of the channels.
pub struct ClientAdapter {
    request_receiver: mpsc::Receiver<(MethodCall, oneshot::Sender<jsonrpc_core::Output>)>,
}

#[derive(Debug)]
struct ClientInner {
    request_sender: mpsc::Sender<(MethodCall, oneshot::Sender<jsonrpc_core::Output>)>,
}

#[derive(Serialize, Deserialize)]
#[serde(untagged)]
enum Message {
    Request(Request),
    Response(jsonrpc_core::Output),
}

pub async fn run_with_client(request_handler: &IoHandler, adapter: ClientAdapter) -> std::io::Result<()> {
    run_with_io(request_handler, tokio::io::stdin(), tokio::io::stdout(), adapter).await
}

pub async fn run(request_handler: &IoHandler) -> std::io::Result<()> {
    let (_client, client_adapter) = Client::new();
    run_with_io(request_handler, tokio::io::stdin(), tokio::io::stdout(), client_adapter).await
}

async fn run_with_io(
    handler: &IoHandler,
    input: impl AsyncRead + Unpin,
    output: impl AsyncWrite + Unpin,
    mut client_adapter: ClientAdapter,
) -> std::io::Result<()> {
    let input = tokio::io::BufReader::new(input);
    let mut input_lines = input.lines();
    let mut output = tokio::io::BufWriter::new(output);
    let mut in_flight: HashMap<jsonrpc_core::Id, oneshot::Sender<_>> = HashMap::new();

    loop {
        tokio::select! {
            next_line = input_lines.next_line() => {
                let next_line = if let Some(next_line) = next_line? {
                    next_line
                } else {
                    return Ok(())
                };

                match serde_json::from_str::<Message>(&next_line)? {
                    Message::Request(request) => {
                        let response = handle_request(handler, request).await;
                        output.write_all(response.as_bytes()).await?;
                        output.write_all(b"\n").await?;
                        output.flush().await?;
                    }
                    Message::Response(response) => {
                        if let Some(chan) = in_flight.remove(response.id()) {
                            chan.send(response).expect("Response channel broken");
                        }
                    }
                }
            }
            next_request = client_adapter.request_receiver.recv() => {
                let (next_request, channel) = next_request.unwrap();
                in_flight.insert(next_request.id.clone(), channel);
                let request_json = serde_json::to_string(&next_request)?;
                output.write_all(request_json.as_bytes()).await?;
                output.write_all(b"\n").await?;
                output.flush().await?;
            }
        }
    }
}

/// Process a request asynchronously
async fn handle_request(io: &IoHandler, input: Request) -> String {
    let response = io.handle_rpc_request(input).await;
    serde_json::to_string(&response).unwrap()
}
