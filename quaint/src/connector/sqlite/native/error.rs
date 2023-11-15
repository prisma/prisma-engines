use crate::connector::sqlite::error::SqliteError;

use crate::error::*;

impl std::fmt::Display for SqliteError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Error code {}: {}",
            self.extended_code,
            rusqlite::ffi::code_to_str(self.extended_code)
        )
    }
}

impl From<rusqlite::Error> for Error {
    fn from(e: rusqlite::Error) -> Error {
        match e {
            rusqlite::Error::ToSqlConversionFailure(error) => match error.downcast::<Error>() {
                Ok(error) => *error,
                Err(error) => {
                    let mut builder = Error::builder(ErrorKind::QueryError(error));

                    builder.set_original_message("Could not interpret parameters in an SQLite query.");

                    builder.build()
                }
            },
            rusqlite::Error::InvalidQuery => {
                let mut builder = Error::builder(ErrorKind::QueryError(e.into()));

                builder.set_original_message(
                    "Could not interpret the query or its parameters. Check the syntax and parameter types.",
                );

                builder.build()
            }
            rusqlite::Error::ExecuteReturnedResults => {
                let mut builder = Error::builder(ErrorKind::QueryError(e.into()));
                builder.set_original_message("Execute returned results, which is not allowed in SQLite.");

                builder.build()
            }

            rusqlite::Error::QueryReturnedNoRows => Error::builder(ErrorKind::NotFound).build(),

            rusqlite::Error::SqliteFailure(rusqlite::ffi::Error { code: _, extended_code }, message) => {
                SqliteError::new(extended_code, message).into()
            }

            rusqlite::Error::SqlInputError {
                error: rusqlite::ffi::Error { extended_code, .. },
                msg,
                ..
            } => SqliteError::new(extended_code, Some(msg)).into(),

            e => Error::builder(ErrorKind::QueryError(e.into())).build(),
        }
    }
}

impl From<rusqlite::types::FromSqlError> for Error {
    fn from(e: rusqlite::types::FromSqlError) -> Error {
        Error::builder(ErrorKind::ColumnReadFailure(e.into())).build()
    }
}
