pub mod calculate_datamodel;

use failure::Error;

pub type SqlIntrospectionResult<T> = core::result::Result<T, Error>;
