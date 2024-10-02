use std::str::FromStr;

use async_tungstenite::{
    tokio::connect_async,
    tungstenite::{
        self,
        client::IntoClientRequest,
        http::{HeaderValue, StatusCode},
        Error as TungsteniteError,
    },
};
use futures::FutureExt;
use tokio_postgres::{Client, Config, NoTls};
use ws_stream_tungstenite::WsStream;

use crate::{
    connector::PostgresWebSocketUrl,
    error::{self, Error, ErrorKind, Name, NativeErrorKind},
};

const CONNECTION_PARAMS_HEADER: &str = "Prisma-Connection-Parameters";

pub(crate) async fn connect_via_websocket(url: PostgresWebSocketUrl) -> crate::Result<Client> {
    let (ws_stream, response) = connect_async(url).await.inspect_err(|e| {
        eprintln!("{}", e);
    })?;

    let Some(header) = response.headers().get(CONNECTION_PARAMS_HEADER) else {
        let message = format!("Missing response header {CONNECTION_PARAMS_HEADER}");
        let error = Error::builder(ErrorKind::Native(NativeErrorKind::ConnectionError(message.into()))).build();
        return Err(error);
    };

    let connection_params = header.to_str().map_err(|inner| {
        Error::builder(ErrorKind::Native(NativeErrorKind::ConnectionError(Box::new(inner)))).build()
    })?;

    let config = Config::from_str(connection_params)?;
    let ws_byte_stream = WsStream::new(ws_stream);

    let (client, connection) = config.connect_raw(ws_byte_stream, NoTls).await?;
    tokio::spawn(connection.map(|r| match r {
        Ok(_) => (),
        Err(e) => {
            tracing::error!("Error in PostgreSQL connection: {:?}", e);
        }
    }));
    Ok(client)
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
