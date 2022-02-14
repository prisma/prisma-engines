#![deny(rust_2018_idioms, unsafe_code, missing_docs)]

//! This crate implements JSON-RPC over standard IO. It uses tokio for async IO, and jsonrpc_core
//! for the JSON-RPC part.
//!
//! Notifications in the Client are not supported yet.

use jsonrpc_core::{IoHandler, MethodCall, Notification, Request};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    io,
    sync::atomic::{AtomicU64, Ordering},
};
use tokio::{
    io::{AsyncBufReadExt, AsyncRead, AsyncWrite, AsyncWriteExt},
    sync::{mpsc, oneshot},
};

static REQUEST_ID_COUNTER: AtomicU64 = AtomicU64::new(0);

/// A handle to a connected client you can use to send requests.
#[derive(Debug, Clone)]
pub struct Client {
    notification_sender: mpsc::Sender<Notification>,
    request_sender: mpsc::Sender<(MethodCall, oneshot::Sender<jsonrpc_core::Output>)>,
}

/// Constructor a JSON-RPC client. Returns a tuple: the client you can use to send requests, and
/// the adapter you must pass to `run_with_client()` to connect the client to the proper IO.
pub fn new_client() -> (Client, ClientAdapter) {
    let (request_sender, request_receiver) = mpsc::channel(30);
    let (notification_sender, notification_receiver) = mpsc::channel(30);
    let client = Client {
        request_sender,
        notification_sender,
    };

    let adapter = ClientAdapter {
        request_receiver,
        notification_receiver,
    };

    (client, adapter)
}

impl Client {
    /// Asynchronously send a JSON-RPC notification.
    pub async fn notify<Req>(&self, method: String, params: Req) -> jsonrpc_core::Result<()>
    where
        Req: serde::Serialize,
    {
        let json_params = serde_json::to_value(params).map_err(|_err| jsonrpc_core::Error::invalid_request())?;
        let params = match json_params {
            jsonrpc_core::Value::Array(arr) => jsonrpc_core::Params::Array(arr),
            jsonrpc_core::Value::Object(obj) => jsonrpc_core::Params::Map(obj),
            _ => return Err(jsonrpc_core::Error::invalid_request()),
        };
        let notification = jsonrpc_core::Notification {
            jsonrpc: Some(jsonrpc_core::Version::V2),
            method,
            params,
        };
        self.notification_sender.send(notification).await.unwrap();
        Ok(())
    }

    /// Asynchronously send a JSON-RPC request.
    pub async fn call<Req, Res>(&self, method: String, params: Req) -> jsonrpc_core::Result<Res>
    where
        Req: serde::Serialize,
        Res: serde::de::DeserializeOwned,
    {
        let id = REQUEST_ID_COUNTER.fetch_add(1, Ordering::Relaxed);
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
        self.request_sender.send((request, response_sender)).await.unwrap();

        let response = response_receiver.await.unwrap();

        match response {
            jsonrpc_core::Output::Success(res) => Ok(serde_json::from_value(res.result).unwrap()),
            jsonrpc_core::Output::Failure(res) => Err(res.error),
        }
    }
}

/// The other side of the channels. Only used as a handle to be passed into run_with_client().
pub struct ClientAdapter {
    request_receiver: mpsc::Receiver<(MethodCall, oneshot::Sender<jsonrpc_core::Output>)>,
    notification_receiver: mpsc::Receiver<Notification>,
}

#[derive(Serialize, Deserialize)]
#[serde(untagged)]
enum Message {
    Response(jsonrpc_core::Output),
    Request(Request),
}

/// Start doing JSON-RPC over stdio. The future will only return once stdin is closed or another
/// IO error happens.
pub async fn run_with_client(request_handler: &IoHandler, adapter: ClientAdapter) -> std::io::Result<()> {
    run_with_io(request_handler, tokio::io::stdin(), tokio::io::stdout(), adapter).await
}

/// Start doing JSON-RPC over stdio, server-only. The future will only return once stdin is closed
/// or another IO error happens.
pub async fn run(request_handler: &IoHandler) -> std::io::Result<()> {
    let (_client, client_adapter) = new_client();
    run_with_io(request_handler, tokio::io::stdin(), tokio::io::stdout(), client_adapter).await
}

async fn run_with_io(
    handler: &IoHandler,
    input: impl AsyncRead + Unpin,
    output: impl AsyncWrite + Send + Unpin + 'static,
    mut client_adapter: ClientAdapter,
) -> std::io::Result<()> {
    let input = tokio::io::BufReader::new(input);
    let mut input_lines = input.lines();
    let mut output = tokio::io::BufWriter::new(output);
    let mut in_flight: HashMap<jsonrpc_core::Id, oneshot::Sender<_>> = HashMap::new();
    let (mut stdout_sender, mut stdout_receiver) = mpsc::channel::<Vec<u8>>(30);

    // Spawn stdout in its own task to queue writes.
    tokio::spawn(async move {
        while let Some(line) = stdout_receiver.recv().await {
            output.write_all(&line).await.unwrap();
            output.write_all(b"\n").await.unwrap();
            output.flush().await.unwrap();
        }
    });

    loop {
        tokio::select! {
            next_line = input_lines.next_line() => {
                handle_stdin_next_line(next_line, stdout_sender.clone(), handler, &mut in_flight).await?;
            }
            next_request = client_adapter.request_receiver.recv() => {
                handle_next_client_request(next_request, &mut stdout_sender, &mut in_flight).await?;
            }
            next_notification = client_adapter.notification_receiver.recv() => {
                handle_next_client_notification(next_notification, &mut stdout_sender).await?;
            }
        }
    }
}

async fn handle_next_client_request(
    next_request: Option<(jsonrpc_core::MethodCall, oneshot::Sender<jsonrpc_core::Output>)>,
    stdout_sender: &mut mpsc::Sender<Vec<u8>>,
    in_flight: &mut HashMap<jsonrpc_core::Id, oneshot::Sender<jsonrpc_core::Output>>,
) -> io::Result<()> {
    let (next_request, channel) = next_request.unwrap();
    in_flight.insert(next_request.id.clone(), channel);
    let request_json = serde_json::to_vec(&next_request)?;
    stdout_sender.send(request_json).await.unwrap();
    Ok(())
}

async fn handle_next_client_notification(
    next_notification: Option<Notification>,
    stdout_sender: &mut mpsc::Sender<Vec<u8>>,
) -> io::Result<()> {
    let next_notification = next_notification.unwrap();
    let request_json = serde_json::to_vec(&next_notification)?;
    stdout_sender.send(request_json).await.unwrap();
    Ok(())
}

async fn handle_stdin_next_line(
    next_line: io::Result<Option<String>>,
    stdout_sender: mpsc::Sender<Vec<u8>>,
    handler: &IoHandler,
    in_flight: &mut HashMap<jsonrpc_core::Id, oneshot::Sender<jsonrpc_core::Output>>,
) -> io::Result<()> {
    let next_line = if let Some(next_line) = next_line? {
        next_line
    } else {
        return Ok(());
    };

    match serde_json::from_str::<Message>(&next_line)? {
        Message::Request(request) => {
            let handler = handler.clone();
            tokio::spawn(async move {
                let response = handle_request(&handler, request).await;
                stdout_sender.send(response.into_bytes()).await.unwrap();
            });
        }
        Message::Response(response) => {
            if let Some(chan) = in_flight.remove(response.id()) {
                chan.send(response).expect("Response channel broken");
            }
        }
    }

    Ok(())
}

/// Process a request asynchronously
async fn handle_request(io: &IoHandler, input: Request) -> String {
    let response = io.handle_rpc_request(input).await;
    serde_json::to_string(&response).unwrap()
}
