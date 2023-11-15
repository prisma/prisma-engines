#![cfg_attr(target_arch = "wasm32", allow(dead_code))]

use crate::error::{Error, ErrorKind};
use std::{convert::TryFrom, path::Path, time::Duration};

pub(crate) const DEFAULT_SQLITE_SCHEMA_NAME: &str = "main";

/// Wraps a connection url and exposes the parsing logic used by Quaint,
/// including default values.
#[derive(Debug)]
pub struct SqliteParams {
    pub connection_limit: Option<usize>,
    /// This is not a `PathBuf` because we need to `ATTACH` the database to the path, and this can
    /// only be done with UTF-8 paths.
    pub file_path: String,
    pub db_name: String,
    pub socket_timeout: Option<Duration>,
    pub max_connection_lifetime: Option<Duration>,
    pub max_idle_connection_lifetime: Option<Duration>,
}

impl TryFrom<&str> for SqliteParams {
    type Error = Error;

    fn try_from(path: &str) -> crate::Result<Self> {
        let path = if path.starts_with("file:") {
            path.trim_start_matches("file:")
        } else {
            path.trim_start_matches("sqlite:")
        };

        let path_parts: Vec<&str> = path.split('?').collect();
        let path_str = path_parts[0];
        let path = Path::new(path_str);

        if path.is_dir() {
            Err(Error::builder(ErrorKind::DatabaseUrlIsInvalid(path.to_str().unwrap().to_string())).build())
        } else {
            let mut connection_limit = None;
            let mut socket_timeout = None;
            let mut max_connection_lifetime = None;
            let mut max_idle_connection_lifetime = None;

            if path_parts.len() > 1 {
                let params = path_parts.last().unwrap().split('&').map(|kv| {
                    let splitted: Vec<&str> = kv.split('=').collect();
                    (splitted[0], splitted[1])
                });

                for (k, v) in params {
                    match k {
                        "connection_limit" => {
                            let as_int: usize = v
                                .parse()
                                .map_err(|_| Error::builder(ErrorKind::InvalidConnectionArguments).build())?;

                            connection_limit = Some(as_int);
                        }
                        "socket_timeout" => {
                            let as_int = v
                                .parse()
                                .map_err(|_| Error::builder(ErrorKind::InvalidConnectionArguments).build())?;

                            socket_timeout = Some(Duration::from_secs(as_int));
                        }
                        "max_connection_lifetime" => {
                            let as_int = v
                                .parse()
                                .map_err(|_| Error::builder(ErrorKind::InvalidConnectionArguments).build())?;

                            if as_int == 0 {
                                max_connection_lifetime = None;
                            } else {
                                max_connection_lifetime = Some(Duration::from_secs(as_int));
                            }
                        }
                        "max_idle_connection_lifetime" => {
                            let as_int = v
                                .parse()
                                .map_err(|_| Error::builder(ErrorKind::InvalidConnectionArguments).build())?;

                            if as_int == 0 {
                                max_idle_connection_lifetime = None;
                            } else {
                                max_idle_connection_lifetime = Some(Duration::from_secs(as_int));
                            }
                        }
                        _ => {
                            tracing::trace!(message = "Discarding connection string param", param = k);
                        }
                    };
                }
            }

            Ok(Self {
                connection_limit,
                file_path: path_str.to_owned(),
                db_name: DEFAULT_SQLITE_SCHEMA_NAME.to_owned(),
                socket_timeout,
                max_connection_lifetime,
                max_idle_connection_lifetime,
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sqlite_params_from_str_should_resolve_path_correctly_with_file_scheme() {
        let path = "file:dev.db";
        let params = SqliteParams::try_from(path).unwrap();
        assert_eq!(params.file_path, "dev.db");
    }

    #[test]
    fn sqlite_params_from_str_should_resolve_path_correctly_with_sqlite_scheme() {
        let path = "sqlite:dev.db";
        let params = SqliteParams::try_from(path).unwrap();
        assert_eq!(params.file_path, "dev.db");
    }

    #[test]
    fn sqlite_params_from_str_should_resolve_path_correctly_with_no_scheme() {
        let path = "dev.db";
        let params = SqliteParams::try_from(path).unwrap();
        assert_eq!(params.file_path, "dev.db");
    }
}
