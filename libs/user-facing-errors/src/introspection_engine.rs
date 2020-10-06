use serde::Serialize;
use user_facing_error_macros::*;

#[derive(Debug, UserFacingError, Serialize)]
#[user_facing(
    code = "P4000",
    message = "Introspection operation failed to produce a schema file: {introspection_error}"
)]
pub struct IntrospectionFailed {
    /// Generic error received from the introspection engine. Indicator of why an introspection failed.
    pub introspection_error: String,
}

#[derive(Debug, UserFacingError, Serialize)]
#[user_facing(code = "P4001", message = "The introspected database was empty: {connection_string}")]
pub struct IntrospectionResultEmpty {
    /// There were no models and no enums detected in the database.
    pub connection_string: String,
}

#[derive(Debug, UserFacingError, Serialize)]
#[user_facing(
    code = "P4002",
    message = "The schema of the introspected database was inconsistent: {explanation}"
)]
pub struct DatabaseSchemaInconsistent {
    /// The schema was inconsistent and therefore introspection failed.
    pub explanation: String,
}
