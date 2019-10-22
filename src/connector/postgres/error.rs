use crate::error::Error;

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
                let reason = format!("{}", e);

                match reason.as_str() {
                    "error connecting to server: timed out" => Error::ConnectTimeout, // sigh...
                    _ => Error::QueryError(e.into()),
                }
            }
        }
    }
}

impl From<native_tls::Error> for Error {
    fn from(e: native_tls::Error) -> Error {
        Error::ConnectionError(e.into())
    }
}
