use crate::error::{Error, DatabaseConstraint};

impl From<tokio_postgres::error::Error> for Error {
    fn from(e: tokio_postgres::error::Error) -> Error {
        use tokio_postgres::error::DbError;

        match e.code().map(|c| c.code()) {
            // Don't look at me, I'm hideous ;((
            Some("23505") => {
                let error = e.into_source().unwrap(); // boom
                let db_error = error.downcast_ref::<DbError>().unwrap(); // BOOM
                let detail = db_error.detail().unwrap(); // KA-BOOM

                let splitted: Vec<&str> = detail.split(")=(").collect();
                let splitted: Vec<&str> = splitted[0].split(" (").collect();

                let field_names = splitted[1].replace("\"", "");
                let field_names: Vec<String> = field_names.split(", ").map(|s| s.to_string()).collect();

                Error::UniqueConstraintViolation {
                    constraint: DatabaseConstraint::Fields(field_names)
                }
            }
            // Even lipstick will not save this...
            Some("23502") => {
                let error = e.into_source().unwrap(); // boom
                let db_error = error.downcast_ref::<DbError>().unwrap(); // BOOM
                let detail = db_error.message(); // KA-BOOM

                let splitted: Vec<&str> = detail.split(' ').collect();
                let field_name = splitted[4].replace("\"", "");

                Error::NullConstraintViolation {
                    constraint: DatabaseConstraint::Fields(vec![field_name])
                }
            }
            Some("3D000") => {
                let error = e.into_source().unwrap(); // boom
                let db_error = error.downcast_ref::<DbError>().unwrap(); // BOOM
                let message = db_error.message();

                let splitted: Vec<&str> = message.split_whitespace().collect();
                let splitted: Vec<&str> = splitted[1].split('"').collect();
                let db_name = splitted[1].into();

                Error::DatabaseDoesNotExist { db_name }
            }
            Some("28P01") => {
                let error = e.into_source().unwrap(); // boom
                let db_error = error.downcast_ref::<DbError>().unwrap(); // BOOM
                let message = db_error.message();

                let splitted: Vec<&str> = message.split_whitespace().collect();
                let splitted: Vec<&str> = splitted.last().unwrap().split('"').collect();
                let user = splitted[1].into();

                Error::AuthenticationFailed { user }
            }
            Some("42P04") => {
                let error = e.into_source().unwrap(); // boom
                let db_error = error.downcast_ref::<DbError>().unwrap(); // BOOM
                let message = db_error.message();

                let splitted: Vec<&str> = message.split_whitespace().collect();
                let splitted: Vec<&str> = splitted[1].split('"').collect();
                let db_name = splitted[1].into();

                Error::DatabaseAlreadyExists { db_name }
            }
            _ => {
                // This is necessary, on top of the other conversions, for the cases where a
                // native_tls error comes wrapped in a tokio_postgres error.
                if let Some(tls_error) = try_extracting_tls_error(&e) {
                    return tls_error;
                }

                // Same for IO errors.
                if let Some(io_error) = try_extracting_io_error(&e) {
                    return io_error;
                }

                let reason = format!("{}", e);

                match reason.as_str() {
                    "error connecting to server: timed out" => Error::ConnectTimeout, // sigh...
                    // https://github.com/sfackler/rust-postgres/blob/0c84ed9f8201f4e5b4803199a24afa2c9f3723b2/tokio-postgres/src/connect_tls.rs#L37
                    "error performing TLS handshake: server does not support TLS" => {
                        Error::TlsError { message: reason }
                    } // double sigh
                    _ => Error::QueryError(e.into()),
                }
            }
        }
    }
}

fn try_extracting_tls_error(err: &tokio_postgres::error::Error) -> Option<Error> {
    use std::error::Error;

    err.source()
        .and_then(|err| err.downcast_ref::<native_tls::Error>())
        .map(|err| err.into())
}

fn try_extracting_io_error(err: &tokio_postgres::error::Error) -> Option<Error> {
    use std::error::Error as _;

    err.source()
        .and_then(|err| err.downcast_ref::<std::io::Error>())
        .map(|err| Error::ConnectionError(Box::new(std::io::Error::new(
            err.kind(),
            format!("{}", err),
        ))))
}

impl From<native_tls::Error> for Error {
    fn from(e: native_tls::Error) -> Error {
        Error::from(&e)
    }
}

impl From<&native_tls::Error> for Error {
    fn from(e: &native_tls::Error) -> Error {
        Error::TlsError {
            message: format!("{}", e),
        }
    }
}
