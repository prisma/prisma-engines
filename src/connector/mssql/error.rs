use crate::error::{DatabaseConstraint, Error, ErrorKind};

impl From<tiberius::error::Error> for Error {
    fn from(e: tiberius::error::Error) -> Error {
        match e {
            tiberius::error::Error::Tls(message) => {
                let message = format!(
                    "The TLS settings didn't allow the connection to be established. Please review your connection string. (error: {})",
                    message
                );

                Error::builder(ErrorKind::TlsError { message }).build()
            }
            tiberius::error::Error::Server(e) if e.code() == 18456 => {
                let user = e.message().split('\'').nth(1).unwrap().to_string();
                let mut builder = Error::builder(ErrorKind::AuthenticationFailed { user });

                builder.set_original_code(format!("{}", e.code()));
                builder.set_original_message(e.message().to_string());

                builder.build()
            }
            tiberius::error::Error::Server(e) if e.code() == 4060 => {
                let db_name = e.message().split('"').nth(1).unwrap().to_string();
                let mut builder = Error::builder(ErrorKind::DatabaseDoesNotExist { db_name });

                builder.set_original_code(format!("{}", e.code()));
                builder.set_original_message(e.message().to_string());

                builder.build()
            }
            tiberius::error::Error::Server(e) if e.code() == 515 => {
                let mut splitted = e.message().split_whitespace();
                let mut splitted = splitted.nth(7).unwrap().split('\'');
                let column = splitted.nth(1).unwrap().to_string();

                let mut builder = Error::builder(ErrorKind::NullConstraintViolation {
                    constraint: DatabaseConstraint::Fields(vec![column]),
                });

                builder.set_original_code(format!("{}", e.code()));
                builder.set_original_message(e.message().to_string());

                builder.build()
            }
            tiberius::error::Error::Server(e) if e.code() == 1801 => {
                let db_name = e.message().split('\'').nth(1).unwrap().to_string();

                let mut builder = Error::builder(ErrorKind::DatabaseAlreadyExists { db_name });

                builder.set_original_code(format!("{}", e.code()));
                builder.set_original_message(e.message().to_string());

                builder.build()
            }
            tiberius::error::Error::Server(e) if e.code() == 2627 => {
                let index = e
                    .message()
                    .split(". ")
                    .nth(1)
                    .unwrap()
                    .split(' ')
                    .last()
                    .unwrap()
                    .split('\'')
                    .nth(1)
                    .unwrap();

                let mut builder = Error::builder(ErrorKind::UniqueConstraintViolation {
                    constraint: DatabaseConstraint::Index(index.to_string()),
                });

                builder.set_original_code(format!("{}", e.code()));
                builder.set_original_message(e.message().to_string());

                builder.build()
            }
            tiberius::error::Error::Server(e) if e.code() == 547 => {
                let index = e
                    .message()
                    .split('.')
                    .next()
                    .unwrap()
                    .split_whitespace()
                    .last()
                    .unwrap()
                    .split('\"')
                    .nth(1)
                    .unwrap();

                let mut builder = Error::builder(ErrorKind::ForeignKeyConstraintViolation {
                    constraint: DatabaseConstraint::Index(index.to_string()),
                });

                builder.set_original_code(format!("{}", e.code()));
                builder.set_original_message(e.message().to_string());

                builder.build()
            }
            tiberius::error::Error::Server(e) if e.code() == 1505 => {
                let mut splitted = e.message().split('\'');
                let index = splitted.nth(3).unwrap().to_string();

                let mut builder = Error::builder(ErrorKind::UniqueConstraintViolation {
                    constraint: DatabaseConstraint::Index(index),
                });

                builder.set_original_code(format!("{}", e.code()));
                builder.set_original_message(e.message().to_string());

                builder.build()
            }
            tiberius::error::Error::Server(e) if e.code() == 2601 => {
                let mut splitted = e.message().split_whitespace();
                let mut splitted = splitted.nth(11).unwrap().split('\'');
                let index = splitted.nth(1).unwrap().to_string();

                let mut builder = Error::builder(ErrorKind::UniqueConstraintViolation {
                    constraint: DatabaseConstraint::Index(index),
                });

                builder.set_original_code(format!("{}", e.code()));
                builder.set_original_message(e.message().to_string());

                builder.build()
            }
            tiberius::error::Error::Server(e) if e.code() == 2714 => {
                let db_name = e.message().split('\'').nth(1).unwrap().to_string();
                let mut builder = Error::builder(ErrorKind::DatabaseAlreadyExists { db_name });

                builder.set_original_code(format!("{}", e.code()));
                builder.set_original_message(e.message().to_string());

                builder.build()
            }
            tiberius::error::Error::Server(e) if e.code() == 2628 => {
                let column_name = e.message().split('\'').nth(3).unwrap().to_string();

                let mut builder = Error::builder(ErrorKind::LengthMismatch {
                    column: Some(column_name),
                });

                builder.set_original_code(format!("{}", e.code()));
                builder.set_original_message(e.message().to_string());

                builder.build()
            }
            tiberius::error::Error::Server(e) if e.code() == 208 => {
                let splitted: Vec<&str> = e.message().split_whitespace().collect();
                let splitted: Vec<&str> = splitted[3].split('\'').collect();
                let table = splitted[1].to_string();

                let mut builder = Error::builder(ErrorKind::TableDoesNotExist { table });
                builder.set_original_code(format!("{}", e.code()));
                builder.set_original_message(e.message().to_string());

                builder.build()
            }
            tiberius::error::Error::Server(e) if e.code() == 207 => {
                let splitted: Vec<&str> = e.message().split_whitespace().collect();
                let splitted: Vec<&str> = splitted[3].split('\'').collect();
                let column = splitted[1].to_string();

                let mut builder = Error::builder(ErrorKind::ColumnNotFound { column });
                builder.set_original_code(format!("{}", e.code()));
                builder.set_original_message(e.message().to_string());

                builder.build()
            }
            tiberius::error::Error::Server(e) => {
                let kind = ErrorKind::QueryError(e.clone().into());

                let mut builder = Error::builder(kind);
                builder.set_original_code(format!("{}", e.code()));
                builder.set_original_message(e.message().to_string());

                builder.build()
            }
            e => Error::builder(ErrorKind::QueryError(e.into())).build(),
        }
    }
}
