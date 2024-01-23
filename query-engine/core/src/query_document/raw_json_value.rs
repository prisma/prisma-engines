use serde::Serialize;
use serde_json::value::RawValue;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RawJson {
    value: String,
}

impl RawJson {
    pub fn from_string(value: String) -> Self {
        Self { value }
    }
}

impl Serialize for RawJson {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let raw_value = RawValue::from_string(self.value.to_owned()).map_err(serde::ser::Error::custom)?;
        raw_value.serialize(serializer)
    }
}
