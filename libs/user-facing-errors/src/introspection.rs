use serde::Serialize;
use user_facing_error_macros::*;

#[derive(Debug, UserFacingError, Serialize)]
#[user_facing(
    code = "P4000",
    message = "Introspection operation failed to produce a schema file: ${introspection_error}"
)]
pub struct IntrospectionFailed {
    /// Generic error received from the introspection engine. Indicator of why an introspection failed.
    pub introspection_error: String,
}
