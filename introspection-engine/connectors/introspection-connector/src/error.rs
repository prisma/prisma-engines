use failure::{format_err, Error, Fail};
use std::fmt::Display;
use user_facing_errors::KnownError;

#[derive(Debug, Fail)]
#[fail(display = "{}", kind)]
pub struct ConnectorError {
    pub user_facing: Option<KnownError>,
    pub kind: ErrorKind,
}

impl ConnectorError {
    pub fn url_parse_error(err: impl Display, url: &str) -> Self {
        ConnectorError {
            user_facing: None,
            kind: ErrorKind::Generic(format_err!(
                "Could not parse the database connection string `{}`: {}",
                url,
                err
            )),
        }
    }
}

#[derive(Debug, Fail)]
pub enum ErrorKind {
    #[fail(display = "{}", _0)]
    Generic(Error),
}
