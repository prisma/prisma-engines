use std::{
    io::{Error as IoError, ErrorKind as IoErrorKind},
    pin::Pin,
    str::FromStr,
    task::{Context, Poll, ready},
};

use bytes::Bytes;
use futures::{FutureExt, Sink, SinkExt, Stream};
use pin_project::pin_project;
use postgres_native_tls::TlsConnector;
use tokio::{
    io::{AsyncBufRead, AsyncRead, AsyncWrite, ReadBuf},
    net::TcpStream,
    task::JoinHandle,
};
use tokio_postgres::{Client, Config};
use tokio_tungstenite::{
    MaybeTlsStream, WebSocketStream, connect_async,
    tungstenite::{
        self, Error as TungsteniteError, Message,
        client::IntoClientRequest,
        http::{HeaderMap, HeaderValue, StatusCode},
    },
};
use tokio_util::io::StreamReader;
use tracing_futures::WithSubscriber;

use crate::{
    connector::PostgresWebSocketUrl,
    error::{self, Error, ErrorKind, Name, NativeErrorKind},
};

const CONNECTION_PARAMS_HEADER: &str = "Prisma-Connection-Parameters";
const HOST_HEADER: &str = "Prisma-Db-Host";

pub(crate) async fn connect_via_websocket(url: PostgresWebSocketUrl) -> crate::Result<(Client, JoinHandle<()>)> {
    let db_name = url.overriden_db_name().map(ToOwned::to_owned);
    let (ws_stream, response) = connect_async(url).await?;

    let connection_params = require_header_value(response.headers(), CONNECTION_PARAMS_HEADER)?;
    let db_host = require_header_value(response.headers(), HOST_HEADER)?;

    let mut config = Config::from_str(connection_params)?;
    if let Some(db_name) = db_name {
        config.dbname(&db_name);
    }
    let ws_byte_stream = WsTunnel::new(ws_stream);

    let tls = TlsConnector::new(native_tls::TlsConnector::new()?, db_host);
    let (client, connection) = config.connect_raw(ws_byte_stream, tls).await?;

    let handle = tokio::spawn(
        connection
            .map(move |result| {
                if let Err(err) = result {
                    tracing::error!("Error in PostgreSQL WebSocket connection: {err:?}");
                }
            })
            .with_current_subscriber(),
    );

    Ok((client, handle))
}

fn require_header_value<'a>(headers: &'a HeaderMap, name: &str) -> crate::Result<&'a str> {
    let Some(header) = headers.get(name) else {
        let message = format!("Missing response header {name}");
        let error = Error::builder(ErrorKind::Native(NativeErrorKind::ConnectionError(message.into()))).build();
        return Err(error);
    };

    let value = header.to_str().map_err(|inner| {
        Error::builder(ErrorKind::Native(NativeErrorKind::ConnectionError(Box::new(inner)))).build()
    })?;

    Ok(value)
}

impl IntoClientRequest for PostgresWebSocketUrl {
    fn into_client_request(self) -> tungstenite::Result<tungstenite::handshake::client::Request> {
        let mut request = self.url.to_string().into_client_request()?;
        let bearer = format!("Bearer {}", self.api_key());
        let auth_header = HeaderValue::from_str(&bearer)?;
        request.headers_mut().insert("Authorization", auth_header);
        Ok(request)
    }
}

impl From<TungsteniteError> for error::Error {
    fn from(value: TungsteniteError) -> Self {
        let builder = match value {
            TungsteniteError::Tls(tls_error) => Error::builder(ErrorKind::Native(NativeErrorKind::TlsError {
                message: tls_error.to_string(),
            })),

            TungsteniteError::Http(response) if response.status() == StatusCode::UNAUTHORIZED => {
                Error::builder(ErrorKind::DatabaseAccessDenied {
                    db_name: Name::Unavailable,
                })
            }

            _ => Error::builder(ErrorKind::Native(NativeErrorKind::ConnectionError(Box::new(value)))),
        };

        builder.build()
    }
}

#[pin_project]
struct WsTunnel {
    #[pin]
    inner: StreamReader<WsBytesStream, Bytes>,
    write_state: WriteState,
}

enum WriteState {
    Free,
    Writing(usize, usize),
}

#[pin_project]
struct WsBytesStream(#[pin] WebSocketStream<MaybeTlsStream<TcpStream>>);

impl WsTunnel {
    fn new(stream: WebSocketStream<MaybeTlsStream<TcpStream>>) -> Self {
        WsTunnel {
            inner: StreamReader::new(WsBytesStream(stream)),
            write_state: WriteState::Free,
        }
    }
}

impl WsBytesStream {
    fn get_pin_mut(self: Pin<&mut Self>) -> Pin<&mut WebSocketStream<MaybeTlsStream<TcpStream>>> {
        self.project().0
    }
}

impl AsyncRead for WsTunnel {
    fn poll_read(self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &mut ReadBuf<'_>) -> Poll<std::io::Result<()>> {
        self.project().inner.poll_read(cx, buf)
    }
}

impl AsyncBufRead for WsTunnel {
    fn poll_fill_buf(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<&[u8]>> {
        self.project().inner.poll_fill_buf(cx)
    }

    fn consume(self: Pin<&mut Self>, amt: usize) {
        self.project().inner.consume(amt)
    }
}

impl AsyncWrite for WsTunnel {
    fn poll_write(self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &[u8]) -> Poll<std::io::Result<usize>> {
        let this = self.get_mut();
        let sink = &mut this.inner.get_mut().0;
        let to_io_err = |err| IoError::other(err);

        match this.write_state {
            WriteState::Free => {
                ready!(sink.poll_ready_unpin(cx)).map_err(to_io_err)?;
                sink.start_send_unpin(Message::Binary(Bytes::copy_from_slice(buf)))
                    .map_err(to_io_err)?;
                this.write_state = WriteState::Writing(buf.as_ptr() as usize, buf.len());
                cx.waker().wake_by_ref();
                Poll::Pending
            }

            WriteState::Writing(addr, len) => {
                if (buf.as_ptr() as usize, buf.len()) != (addr, len) {
                    return Poll::Ready(Err(IoError::new(
                        IoErrorKind::ResourceBusy,
                        "concurrent writes to the WebSocket tunnel are not allowed",
                    )));
                }
                ready!(sink.poll_flush_unpin(cx)).map_err(to_io_err)?;
                this.write_state = WriteState::Free;
                Poll::Ready(Ok(len))
            }
        }
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        self.project()
            .inner
            .get_pin_mut()
            .get_pin_mut()
            .poll_flush(cx)
            .map_err(IoError::other)
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        self.project()
            .inner
            .get_pin_mut()
            .get_pin_mut()
            .poll_close(cx)
            .map_err(IoError::other)
    }
}

impl Stream for WsBytesStream {
    type Item = Result<Bytes, IoError>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        match self.get_pin_mut().poll_next(cx) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(None) => Poll::Ready(None),
            Poll::Ready(Some(Ok(msg))) => match msg {
                Message::Binary(data) => Poll::Ready(Some(Ok(data))),
                Message::Close(_) => Poll::Ready(None),
                Message::Text(data) => {
                    tracing::warn!(%data, "unexpected text frame in a WebSocket tunnel");
                    cx.waker().wake_by_ref();
                    Poll::Pending
                }
                Message::Ping(_) | Message::Pong(_) => {
                    cx.waker().wake_by_ref();
                    Poll::Pending
                }
                Message::Frame(_) => Poll::Ready(Some(Err(IoError::other("unexpected raw frame")))),
            },
            Poll::Ready(Some(Err(err))) => Poll::Ready(Some(Err(IoError::other(err)))),
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.0.size_hint()
    }
}
