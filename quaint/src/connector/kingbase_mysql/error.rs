use crosstarget_utils::{RegExpCompat, regex::RegExp};
use enumflags2::BitFlags;
use std::fmt::{Display, Formatter};

use crate::error::{DatabaseConstraint, Error, ErrorKind, Name};

#[derive(Debug)]
pub struct KingbaseError {
    pub code: String,
    pub message: String,
    pub severity: String,
    pub detail: Option<String>,
    pub column: Option<String>,
    pub constraint: Option<String>,
    pub hint: Option<String>,
}

impl std::error::Error for KingbaseError {}

impl Display for KingbaseError {
    // copy of tokio_postgres::error::DbError::fmt
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
    let re = RegExp::new(r#"foreign key constraint "([^"]+)""#, BitFlags::empty()).unwrap();
    re.captures(message).and_then(|caps| caps.get(1).cloned())
}

/// The CREATE and COMMENT are sent together. Read the logical name before
/// execution, since a failing CREATE UNIQUE INDEX never installs its comment.
#[cfg(feature = "kingbase-mysql-native")]
pub(crate) fn index_names_from_sql(mut sql: &str) -> Vec<(String, String)> {
    if !sql.contains("prisma:logical-index:") {
        return Vec::new();
    }
    let re = RegExp::new(
        r"^COMMENT ON INDEX `((?:``|[^`])+)` IS 'prisma:logical-index:((?:''|[^'])*)'",
        BitFlags::empty(),
    )
    .unwrap();
    let mut names = Vec::new();
    while !sql.is_empty() {
        // Only inspect SQL tokens, never user literals, identifiers or comments.
        if sql.starts_with("--") || sql.starts_with('#') {
            sql = sql.find('\n').map_or("", |end| &sql[end + 1..]);
        } else if let Some(rest) = sql.strip_prefix("/*") {
            let mut depth = 1;
            sql = rest;
            while !sql.is_empty() && depth > 0 {
                if let Some(rest) = sql.strip_prefix("/*") {
                    depth += 1;
                    sql = rest;
                } else if let Some(rest) = sql.strip_prefix("*/") {
                    depth -= 1;
                    sql = rest;
                } else {
                    sql = &sql[sql.chars().next().unwrap().len_utf8()..];
                }
            }
        } else if matches!(sql.as_bytes()[0], b'\'' | b'"' | b'`') || sql.starts_with("E'") || sql.starts_with("e'") {
            let escapes = sql.starts_with("E'") || sql.starts_with("e'");
            if escapes {
                sql = &sql[1..];
            }
            let quote = sql.as_bytes()[0] as char;
            sql = &sql[1..];
            while !sql.is_empty() {
                let ch = sql.chars().next().unwrap();
                sql = &sql[ch.len_utf8()..];
                if escapes && ch == '\\' {
                    if let Some(next) = sql.chars().next() {
                        sql = &sql[next.len_utf8()..];
                    }
                } else if ch == quote {
                    if sql.starts_with(quote) {
                        sql = &sql[1..];
                    } else {
                        break;
                    }
                }
            }
        } else if sql.starts_with('$')
            && let Some(end) = sql[1..].find('$').map(|end| end + 1)
            && sql[1..end].bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
        {
            let marker = &sql[..=end];
            sql = sql[end + 1..]
                .find(marker)
                .map_or("", |close| &sql[end + 1 + close + marker.len()..]);
        } else if let Some(captures) = re.captures(sql) {
            let physical = captures[1].replace("``", "`");
            if physical.starts_with("_prisma_idx_") && !captures[2].is_empty() {
                names.push((physical, captures[2].replace("''", "'")));
            }
            sql = &sql[captures[0].len()..];
        } else {
            let mut chars = sql.char_indices();
            let (_, first) = chars.next().unwrap();
            let length = if first.is_alphabetic() || first == '_' {
                chars
                    .find(|(_, ch)| !ch.is_alphanumeric() && !matches!(*ch, '_' | '$'))
                    .map_or(sql.len(), |(offset, _)| offset)
            } else {
                first.len_utf8()
            };
            sql = &sql[length..];
        }
    }
    names
}

#[cfg(all(test, feature = "kingbase-mysql-native"))]
mod index_name_tests {
    #[test]
    fn pending_index_comments_ignore_sql_literals_and_preserve_escaped_names() {
        let names = super::index_names_from_sql(
            r#"
            SELECT 'COMMENT ON INDEX `_prisma_idx_fake` IS ''prisma:logical-index:wrong''';
            /* COMMENT ON INDEX `_prisma_idx_fake` IS 'prisma:logical-index:wrong' */
            -- COMMENT ON INDEX `_prisma_idx_fake` IS 'prisma:logical-index:wrong'
            SELECT $body$ COMMENT ON INDEX `_prisma_idx_fake` IS 'prisma:logical-index:wrong' $body$;
            SELECT E'escaped\' COMMENT ON INDEX `_prisma_idx_fake` IS \'prisma:logical-index:wrong\'';
            CREATE UNIQUE INDEX `_prisma_idx_real` ON items(value);
            COMMENT ON INDEX `_prisma_idx_real` IS 'prisma:logical-index:custom''唯一';
            COMMENT ON INDEX `_prisma_idx_other` IS 'prisma:logical-index:second';
        "#,
        );
        assert_eq!(
            names,
            vec![
                ("_prisma_idx_real".into(), "custom'唯一".into()),
                ("_prisma_idx_other".into(), "second".into())
            ]
        );
    }
}

impl From<KingbaseError> for Error {
    fn from(value: KingbaseError) -> Self {
        match value.code.as_str() {
            "22003" => {
                let mut builder = Error::builder(ErrorKind::value_out_of_range(value.message.clone()));

                builder.set_original_code(value.code);
                builder.set_original_message(value.message);

                builder.build()
            }
            "22001" => {
                let mut builder = Error::builder(ErrorKind::LengthMismatch {
                    column: Name::Unavailable,
                });

                builder.set_original_code(&value.code);
                builder.set_original_message(value.to_string());

                builder.build()
            }
            "23505" => {
                let constraint = value
                    .constraint
                    .map(DatabaseConstraint::Index)
                    .or_else(|| {
                        value
                            .detail
                            .as_ref()
                            .and_then(|d| d.split(")=(").next())
                            .and_then(|d| d.split_once(" (").map(|(_, rest)| rest.replace('"', "")))
                            .map(|s| DatabaseConstraint::fields(s.split(", ")))
                    })
                    .unwrap_or(DatabaseConstraint::CannotParse);

                let kind = ErrorKind::UniqueConstraintViolation { constraint };
                let mut builder = Error::builder(kind);

                builder.set_original_code(value.code);

                if let Some(detail) = value.detail {
                    builder.set_original_message(detail);
                }

                builder.build()
            }

            // Even lipstick will not save this...
            "23502" => {
                let constraint = DatabaseConstraint::fields(value.column);

                let kind = ErrorKind::NullConstraintViolation { constraint };
                let mut builder = Error::builder(kind);

                builder.set_original_code(value.code);

                if let Some(detail) = value.detail {
                    builder.set_original_message(detail);
                }

                builder.build()
            }
            "23503" => match value.column {
                Some(column) => {
                    let mut builder = Error::builder(ErrorKind::ForeignKeyConstraintViolation {
                        constraint: DatabaseConstraint::fields(Some(column)),
                    });

                    builder.set_original_code(value.code);
                    builder.set_original_message(value.message);

                    builder.build()
                }
                None => {
                    // `value.message` looks like `update on table "Child" violates foreign key constraint "Child_parent_id_fkey"`
                    let constraint = extract_fk_constraint_name(value.message.as_str())
                        .map(DatabaseConstraint::Index)
                        .unwrap_or(DatabaseConstraint::CannotParse);

                    let kind = ErrorKind::ForeignKeyConstraintViolation { constraint };
                    let mut builder = Error::builder(kind);

                    builder.set_original_code(value.code);
                    builder.set_original_message(value.message);

                    builder.build()
                }
            },
            "3D000" => {
                let db_name = value
                    .message
                    .split_whitespace()
                    .nth(1)
                    .and_then(|s| s.split('"').nth(1))
                    .into();

                let kind = ErrorKind::DatabaseDoesNotExist { db_name };
                let mut builder = Error::builder(kind);

                builder.set_original_code(value.code);
                builder.set_original_message(value.message);

                builder.build()
            }
            "28000" => {
                let db_name = value
                    .message
                    .split_whitespace()
                    .nth(5)
                    .and_then(|s| s.split('"').nth(1))
                    .into();

                let kind = ErrorKind::DatabaseAccessDenied { db_name };
                let mut builder = Error::builder(kind);

                builder.set_original_code(value.code);
                builder.set_original_message(value.message);

                builder.build()
            }
            "28P01" => {
                let message = value.message;

                let user = message
                    .split_whitespace()
                    .last()
                    .and_then(|s| s.split('"').nth(1))
                    .into();

                let kind = ErrorKind::AuthenticationFailed { user };
                let mut builder = Error::builder(kind);

                builder.set_original_code(value.code);
                builder.set_original_message(message);

                builder.build()
            }
            "40001" => {
                let mut builder: crate::error::ErrorBuilder = Error::builder(ErrorKind::TransactionWriteConflict);

                builder.set_original_code(value.code);
                builder.set_original_message(value.message);

                builder.build()
            }
            "42P01" => {
                let table = value
                    .message
                    .split_whitespace()
                    .nth(1)
                    .and_then(|s| s.split('"').nth(1))
                    .into();

                let kind = ErrorKind::TableDoesNotExist { table };
                let mut builder = Error::builder(kind);

                builder.set_original_code(value.code);
                builder.set_original_message(value.message);

                builder.build()
            }
            "42703" => {
                // Kingbase may localize the error message or include the table
                // name before the missing column. Prefer PostgreSQL's structured
                // column field and only fall back to the last quoted identifier.
                let column = value
                    .column
                    .or_else(|| value.message.split('"').rev().nth(1).map(ToOwned::to_owned))
                    .into();

                let kind = ErrorKind::ColumnNotFound { column };
                let mut builder = Error::builder(kind);

                builder.set_original_code(value.code);
                builder.set_original_message(value.message);
                builder.build()
            }

            "42P04" => {
                let db_name = value
                    .message
                    .split_whitespace()
                    .nth(1)
                    .and_then(|s| s.split('"').nth(1))
                    .into();

                let kind = ErrorKind::DatabaseAlreadyExists { db_name };
                let mut builder = Error::builder(kind);

                builder.set_original_code(value.code);
                builder.set_original_message(value.message);

                builder.build()
            }

            "53300" => {
                let code = value.code.to_owned();
                let message = value.to_string();
                let kind = ErrorKind::TooManyConnections(value.into());
                let mut builder = Error::builder(kind);
                builder.set_original_code(code);
                builder.set_original_message(message);
                builder.build()
            }

            _ => {
                let code = value.code.to_owned();
                let message = value.to_string();
                let mut builder = Error::builder(ErrorKind::QueryError(value.into()));

                builder.set_original_code(code);
                builder.set_original_message(message);
                builder.build()
            }
        }
    }
}
