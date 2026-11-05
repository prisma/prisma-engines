use crosstarget_utils::{RegExpCompat, regex::RegExp};
use enumflags2::BitFlags;
use std::fmt::{Display, Formatter};

use crate::error::{DatabaseConstraint, Error, ErrorKind, Name};

#[derive(Debug)]
pub struct KingbaseOracleError {
    pub code: String,
    pub message: String,
    pub severity: String,
    pub detail: Option<String>,
    pub column: Option<String>,
    pub constraint: Option<String>,
    pub hint: Option<String>,
}

impl std::error::Error for KingbaseOracleError {}

impl Display for KingbaseOracleError {
    fn fmt(&self, fmt: &mut Formatter<'_>) -> std::fmt::Result {
        write!(fmt, "{}: {}", self.severity, self.message)?;
        if let Some(detail) = &self.detail {
            write!(fmt, "\nDETAIL: {detail}")?;
        }
        if let Some(hint) = &self.hint {
            write!(fmt, "\nHINT: {hint}")?;
        }
        Ok(())
    }
}

fn extract_fk_constraint_name(message: &str) -> Option<String> {
    let regex = RegExp::new(r#"foreign key constraint "([^"]+)""#, BitFlags::empty()).unwrap();
    regex.captures(message).and_then(|captures| captures.get(1).cloned())
}

impl From<KingbaseOracleError> for Error {
    fn from(value: KingbaseOracleError) -> Self {
        match value.code.as_str() {
            "22003" => with_original(
                ErrorKind::value_out_of_range(value.message.clone()),
                value.code,
                value.message,
            ),
            "22001" => with_original(
                ErrorKind::LengthMismatch {
                    column: Name::Unavailable,
                },
                value.code.clone(),
                value.to_string(),
            ),
            "23505" => {
                let constraint = value
                    .constraint
                    .clone()
                    .map(DatabaseConstraint::Index)
                    .or_else(|| {
                        value
                            .detail
                            .as_ref()
                            .and_then(|detail| detail.split(")=(").next())
                            .and_then(|detail| detail.split_once(" (").map(|(_, rest)| rest.replace('"', "")))
                            .map(|fields| DatabaseConstraint::fields(fields.split(", ")))
                    })
                    .unwrap_or(DatabaseConstraint::CannotParse);
                with_original(
                    ErrorKind::UniqueConstraintViolation { constraint },
                    value.code,
                    value.detail.unwrap_or(value.message),
                )
            }
            "23502" => with_original(
                ErrorKind::NullConstraintViolation {
                    constraint: DatabaseConstraint::fields(value.column),
                },
                value.code,
                value.detail.unwrap_or(value.message),
            ),
            "23503" => {
                let constraint = value
                    .column
                    .map(|column| DatabaseConstraint::fields(Some(column)))
                    .or_else(|| extract_fk_constraint_name(&value.message).map(DatabaseConstraint::Index))
                    .unwrap_or(DatabaseConstraint::CannotParse);
                with_original(
                    ErrorKind::ForeignKeyConstraintViolation { constraint },
                    value.code,
                    value.message,
                )
            }
            "3D000" => {
                let db_name = quoted_word(&value.message, 1).into();
                with_original(ErrorKind::DatabaseDoesNotExist { db_name }, value.code, value.message)
            }
            "28000" => {
                let db_name = quoted_word(&value.message, 5).into();
                with_original(ErrorKind::DatabaseAccessDenied { db_name }, value.code, value.message)
            }
            "28P01" => {
                let user = value
                    .message
                    .split_whitespace()
                    .last()
                    .and_then(|word| word.split('"').nth(1))
                    .into();
                with_original(ErrorKind::AuthenticationFailed { user }, value.code, value.message)
            }
            "40001" => with_original(ErrorKind::TransactionWriteConflict, value.code, value.message),
            "42P01" => {
                let table = quoted_word(&value.message, 1).into();
                with_original(ErrorKind::TableDoesNotExist { table }, value.code, value.message)
            }
            "42703" => {
                let column = value
                    .column
                    .or_else(|| value.message.split('"').rev().nth(1).map(ToOwned::to_owned))
                    .into();
                with_original(ErrorKind::ColumnNotFound { column }, value.code, value.message)
            }
            "42P04" => {
                let db_name = quoted_word(&value.message, 1).into();
                with_original(ErrorKind::DatabaseAlreadyExists { db_name }, value.code, value.message)
            }
            "53300" => {
                let code = value.code.clone();
                let message = value.to_string();
                with_original(ErrorKind::TooManyConnections(value.into()), code, message)
            }
            _ => {
                let code = value.code.clone();
                let message = value.to_string();
                with_original(ErrorKind::QueryError(value.into()), code, message)
            }
        }
    }
}

fn quoted_word(message: &str, index: usize) -> Option<&str> {
    message
        .split_whitespace()
        .nth(index)
        .and_then(|word| word.split('"').nth(1))
}

fn with_original(kind: ErrorKind, code: String, message: String) -> Error {
    let mut builder = Error::builder(kind);
    builder.set_original_code(code);
    builder.set_original_message(message);
    builder.build()
}
