use crate::error::*;

#[derive(Debug)]
pub struct SqliteError {
    pub extended_code: i32,
    pub message: Option<String>,
}

#[cfg(not(feature = "sqlite-native"))]
impl std::fmt::Display for SqliteError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Error code {}", self.extended_code)
    }
}

impl std::error::Error for SqliteError {}

impl SqliteError {
    pub fn new(extended_code: i32, message: Option<String>) -> Self {
        Self { extended_code, message }
    }

    pub fn primary_code(&self) -> i32 {
        self.extended_code & 0xFF
    }
}

impl From<SqliteError> for Error {
    fn from(error: SqliteError) -> Self {
        match error {
            SqliteError {
                extended_code: super::ffi::SQLITE_CONSTRAINT_UNIQUE | super::ffi::SQLITE_CONSTRAINT_PRIMARYKEY,
                message: Some(description),
            } => {
                let constraint = description
                    .split(": ")
                    .nth(1)
                    .map(|s| s.split(", "))
                    .map(|i| i.flat_map(|s| s.split('.').last()))
                    .map(DatabaseConstraint::fields)
                    .unwrap_or(DatabaseConstraint::CannotParse);

                let kind = ErrorKind::UniqueConstraintViolation { constraint };
                let mut builder = Error::builder(kind);

                builder.set_original_code(error.extended_code.to_string());
                builder.set_original_message(description);

                builder.build()
            }

            SqliteError {
                extended_code: super::ffi::SQLITE_CONSTRAINT_NOTNULL,
                message: Some(description),
            } => {
                let constraint = description
                    .split(": ")
                    .nth(1)
                    .map(|s| s.split(", "))
                    .map(|i| i.flat_map(|s| s.split('.').last()))
                    .map(DatabaseConstraint::fields)
                    .unwrap_or(DatabaseConstraint::CannotParse);

                let kind = ErrorKind::NullConstraintViolation { constraint };
                let mut builder = Error::builder(kind);

                builder.set_original_code(error.extended_code.to_string());
                builder.set_original_message(description);

                builder.build()
            }

            SqliteError {
                extended_code: super::ffi::SQLITE_CONSTRAINT_FOREIGNKEY | super::ffi::SQLITE_CONSTRAINT_TRIGGER,
                message: Some(description),
            } => {
                let mut builder = Error::builder(ErrorKind::ForeignKeyConstraintViolation {
                    constraint: DatabaseConstraint::ForeignKey,
                });

                builder.set_original_code(error.extended_code.to_string());
                builder.set_original_message(description);

                builder.build()
            }

            SqliteError { extended_code, message } if error.primary_code() == super::ffi::SQLITE_BUSY => {
                let mut builder = Error::builder(ErrorKind::SocketTimeout);
                builder.set_original_code(format!("{extended_code}"));

                if let Some(description) = message {
                    builder.set_original_message(description);
                }

                builder.build()
            }

            SqliteError {
                extended_code,
                ref message,
            } => match message {
                Some(d) if d.starts_with("no such table") => {
                    let table = d.split(": ").last().into();
                    let kind = ErrorKind::TableDoesNotExist { table };

                    let mut builder = Error::builder(kind);
                    builder.set_original_code(format!("{extended_code}"));
                    builder.set_original_message(d);

                    builder.build()
                }
                Some(d) if d.contains("has no column named") => {
                    let column = d.split(" has no column named ").last().into();
                    let kind = ErrorKind::ColumnNotFound { column };

                    let mut builder = Error::builder(kind);
                    builder.set_original_code(format!("{extended_code}"));
                    builder.set_original_message(d);

                    builder.build()
                }
                Some(d) if d.starts_with("no such column: ") => {
                    let column = d.split("no such column: ").last().into();
                    let kind = ErrorKind::ColumnNotFound { column };

                    let mut builder = Error::builder(kind);
                    builder.set_original_code(format!("{extended_code}"));
                    builder.set_original_message(d);

                    builder.build()
                }
                _ => {
                    let description = message.as_ref().map(|d| d.to_string());
                    let mut builder = Error::builder(ErrorKind::QueryError(error.into()));
                    builder.set_original_code(format!("{extended_code}"));

                    if let Some(description) = description {
                        builder.set_original_message(description);
                    }

                    builder.build()
                }
            },
        }
    }
}
