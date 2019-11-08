use crate::error::Error;
use failure::format_err;

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
                let field_name = splitted[1].replace("\"", "");

                Error::UniqueConstraintViolation { field_name }
            }
            // Even lipstick will not save this...
            Some("23502") => {
                let error = e.into_source().unwrap(); // boom
                let db_error = error.downcast_ref::<DbError>().unwrap(); // BOOM
                let detail = db_error.detail().unwrap(); // KA-BOOM

                let splitted: Vec<&str> = detail.split(")=(").collect();
                let splitted: Vec<&str> = splitted[0].split(" (").collect();
                let field_name = splitted[1].replace("\"", "");

                Error::NullConstraintViolation { field_name }
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
        .map(|err| Error::ConnectionError(format_err!("{}", err)))
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
