use crate::StringFromEnvVar;

pub fn validate_url(name: &str, expected_protocol: &str, url: StringFromEnvVar) -> Result<StringFromEnvVar, String> {
    if url.value.starts_with(expected_protocol) {
        Ok(url)
    } else {
        Err(format!(
            "The URL for datasource `{}` must start with the protocol `{}`.",
            name, expected_protocol
        ))
    }
}
