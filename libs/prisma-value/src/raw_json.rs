use serde::Serialize;
use serde_json::value::RawValue;

/// We are using RawJson object to prevent stringification of
/// certain JSON values. Difference between this is and PrismaValue::Json
/// is the following:
///
/// PrismaValue::Json(r"""{"foo": "bar"}""") when serialized will produce the string "{\"foo\":\"bar\"}".
/// RawJson(r"""{"foo": "bar"}""") will produce {"foo": "bar" } JSON object.
/// So, it essentially would treat provided string as pre-serialized JSON fragment and not a string to be serialized.
///
/// It is a wrapper of `serde_json::value::RawValue`. We don't want to use `RawValue` inside of `ArgumentValue`
/// directly because:
/// 1. We need `Eq` implementation
/// 2. `serde_json::value::RawValue::from_string` may error and we'd like to delay handling of that error to
/// serialization time
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RawJson {
    value: String,
}

impl RawJson {
    pub fn try_new(value: impl Serialize) -> serde_json::Result<Self> {
        Ok(Self {
            value: serde_json::to_string(&value)?,
        })
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
