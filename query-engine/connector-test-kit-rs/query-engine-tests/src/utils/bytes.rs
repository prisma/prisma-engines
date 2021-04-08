use base64::encode;

pub fn string_to_base64(str: &str) -> String {
    encode(str.as_bytes())
}
