use std::str::FromStr;

use async_tungstenite::{
    tokio::connect_async,
    tungstenite::{
        self,
        client::IntoClientRequest,
        http::{HeaderMap, HeaderValue, StatusCode},
        Error as TungsteniteError,
    },
};
use futures::FutureExt;
use postgres_native_tls::TlsConnector;
use prisma_metrics::WithMetricsInstrumentation;
use tokio_postgres::{Client, Config};
use tracing_futures::WithSubscriber;
use ws_stream_tungstenite::WsStream;

use crate::{
    connector::PostgresWebSocketUrl,
    error::{self, Error, ErrorKind, Name, NativeErrorKind},
};

const CONNECTION_PARAMS_HEADER: &str = "Prisma-Connection-Parameters";
const HOST_HEADER: &str = "Prisma-Db-Host";

pub(crate) async fn connect_via_websocket(url: PostgresWebSocketUrl) -> crate::Result<Client> {
    let db_name = url.overriden_db_name().map(ToOwned::to_owned);
    let (ws_stream, response) = connect_async(url).await?;

    let connection_params = require_header_value(response.headers(), CONNECTION_PARAMS_HEADER)?;
    let db_host = require_header_value(response.headers(), HOST_HEADER)?;

    let mut config = Config::from_str(connection_params)?;
    if let Some(db_name) = db_name {
        config.dbname(&db_name);
    }
    let ws_byte_stream = WsStream::new(ws_stream);

    let tls = TlsConnector::new(native_tls::TlsConnector::new()?, db_host);
    let (client, connection) = config.connect_raw(ws_byte_stream, tls).await?;
    tokio::spawn(
        connection
            .map(|r| {
                if let Err(e) = r {
                    tracing::error!("Error in PostgreSQL WebSocket connection: {e:?}");
                }
            })
            .with_current_subscriber()
            .with_current_recorder(),
    );
    Ok(client)
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
