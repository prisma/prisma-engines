use failure::{Error, Fail};

#[derive(Debug, Fail)]
pub enum ConnectorError {
    #[fail(display = "{}", _0)]
    Generic(Error),
}
