use crate::error::{DatabaseConstraint, Error, ErrorKind};

impl From<tokio_postgres::error::Error> for Error {
    fn from(e: tokio_postgres::error::Error) -> Error {
        use tokio_postgres::error::DbError;

        match e.code().map(|c| c.code()) {
            Some(code) if code == "22001" => {
                let code = code.to_string();
                let error = e.into_source().unwrap(); // boom
                let db_error = error.downcast_ref::<DbError>().unwrap(); // BOOM

                let mut builder = Error::builder(ErrorKind::LengthMismatch { column: None });

                builder.set_original_code(code);
                builder.set_original_message(db_error.to_string());

                builder.build()
            }
            // Don't look at me, I'm hideous ;((
            Some(code) if code == "23505" => {
                let code = code.to_string();
                let error = e.into_source().unwrap(); // boom
                let db_error = error.downcast_ref::<DbError>().unwrap(); // BOOM
                let detail = db_error.detail().unwrap(); // KA-BOOM

                let splitted: Vec<&str> = detail.split(")=(").collect();
                let splitted: Vec<&str> = splitted[0].split(" (").collect();

                let field_names = splitted[1].replace("\"", "");
                let field_names: Vec<String> = field_names.split(", ").map(|s| s.to_string()).collect();

                let mut builder = Error::builder(ErrorKind::UniqueConstraintViolation {
                    constraint: DatabaseConstraint::Fields(field_names),
                });

                builder.set_original_code(code);
                builder.set_original_message(detail);

                builder.build()
            }
            // Even lipstick will not save this...
            Some(code) if code == "23502" => {
                let code = code.to_string();
                let error = e.into_source().unwrap(); // boom
                let db_error = error.downcast_ref::<DbError>().unwrap(); // BOOM

                let column_name = db_error
                    .column()
                    .expect("column on null constraint violation error")
                    .to_owned();

                let mut builder = Error::builder(ErrorKind::NullConstraintViolation {
                    constraint: DatabaseConstraint::Fields(vec![column_name]),
                });

                builder.set_original_code(code);
                builder.set_original_message(db_error.message());

                builder.build()
            }
            Some(code) if code == "23503" => {
                let code = code.to_string();
                let error = e.into_source().unwrap(); // boom
                let db_error = error.downcast_ref::<DbError>().unwrap(); // BOOM

                let column_name = db_error
                    .column()
                    .expect("column on null constraint violation error")
                    .to_owned();

                let mut builder = Error::builder(ErrorKind::ForeignKeyConstraintViolation {
                    constraint: DatabaseConstraint::Fields(vec![column_name]),
                });

                builder.set_original_code(code);
                builder.set_original_message(db_error.message());

                builder.build()
            }
            Some(code) if code == "3D000" => {
                let code = code.to_string();
                let error = e.into_source().unwrap(); // boom
                let db_error = error.downcast_ref::<DbError>().unwrap(); // BOOM
                let message = db_error.message();

                let splitted: Vec<&str> = message.split_whitespace().collect();
                let splitted: Vec<&str> = splitted[1].split('"').collect();
                let db_name = splitted[1].into();

                let mut builder = Error::builder(ErrorKind::DatabaseDoesNotExist { db_name });

                builder.set_original_code(code);
                builder.set_original_message(message);

                builder.build()
            }
            Some(code) if code == "28P01" => {
                let code = code.to_string();
                let error = e.into_source().unwrap(); // boom
                let db_error = error.downcast_ref::<DbError>().unwrap(); // BOOM
                let message = db_error.message();

                let splitted: Vec<&str> = message.split_whitespace().collect();
                let splitted: Vec<&str> = splitted.last().unwrap().split('"').collect();
                let user = splitted[1].into();

                let mut builder = Error::builder(ErrorKind::AuthenticationFailed { user });

                builder.set_original_code(code);
                builder.set_original_message(message);

                builder.build()
            }
            Some(code) if code == "42P04" => {
                let code = code.to_string();
                let error = e.into_source().unwrap(); // boom
                let db_error = error.downcast_ref::<DbError>().unwrap(); // BOOM
                let message = db_error.message();

                let splitted: Vec<&str> = message.split_whitespace().collect();
                let splitted: Vec<&str> = splitted[1].split('"').collect();
                let db_name = splitted[1].into();

                let mut builder = Error::builder(ErrorKind::DatabaseAlreadyExists { db_name });

                builder.set_original_code(code);
                builder.set_original_message(message);

                builder.build()
            }
            code => {
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
                    "error connecting to server: timed out" => {
                        let mut builder = Error::builder(ErrorKind::ConnectTimeout(
                            "tokio-postgres timeout connecting to server".into(),
                        ));

                        if let Some(code) = code {
                            builder.set_original_code(code);
                        };

                        builder.set_original_message(reason);
                        builder.build()
                    } // sigh...
                    // https://github.com/sfackler/rust-postgres/blob/0c84ed9f8201f4e5b4803199a24afa2c9f3723b2/tokio-postgres/src/connect_tls.rs#L37
                    "error performing TLS handshake: server does not support TLS" => {
                        let mut builder = Error::builder(ErrorKind::TlsError {
                            message: reason.clone(),
                        });

                        if let Some(code) = code {
                            builder.set_original_code(code);
                        };

                        builder.set_original_message(reason);
                        builder.build()
                    } // double sigh
                    _ => {
                        let code = code.map(|c| c.to_string());
                        let mut builder = Error::builder(ErrorKind::QueryError(e.into()));

                        if let Some(code) = code {
                            builder.set_original_code(code);
                        };

                        builder.set_original_message(reason);
                        builder.build()
                    }
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
        .map(|err| ErrorKind::ConnectionError(Box::new(std::io::Error::new(err.kind(), format!("{}", err)))))
        .map(|kind| Error::builder(kind).build())
}

impl From<native_tls::Error> for Error {
    fn from(e: native_tls::Error) -> Error {
        Error::from(&e)
    }
}

impl From<&native_tls::Error> for Error {
    fn from(e: &native_tls::Error) -> Error {
        let kind = ErrorKind::TlsError {
            message: format!("{}", e),
        };

        Error::builder(kind).build()
    }
}
