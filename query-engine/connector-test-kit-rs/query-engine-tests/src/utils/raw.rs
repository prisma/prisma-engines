use anyhow::Result;
use chrono::{DateTime, FixedOffset};
use prisma_value::encode_bytes;

pub fn fmt_query_raw(query: &str, params: Vec<RawValue>) -> String {
    let params = params_to_json(params);
    let params = serde_json::to_string(&params).unwrap();

    format!(
        r#"mutation {{ queryRaw(query: "{}", parameters: "{}") }}"#,
        query.replace('"', "\\\""),
        params.replace('"', "\\\"")
    )
}

pub fn fmt_execute_raw<T>(query: &str, params: Vec<RawValue>) -> String {
    let params = params_to_json(params);
    let params = serde_json::to_string(&params).unwrap();

    format!(
        r#"mutation {{ executeRaw(query: "{}", parameters: "{}") }}"#,
        query.replace('"', "\\\""),
        params.replace('"', "\\\"")
    )
}

pub enum RawValue {
    DateTime(DateTime<FixedOffset>),
    Bytes(Vec<u8>),
    BigInt(i64),
    Decimal(String),
    Scalar(serde_json::Value),
}

impl RawValue {
    pub fn try_datetime(s: &str) -> Result<Self> {
        Ok(Self::DateTime(DateTime::parse_from_rfc3339(s)?))
    }

    pub fn bytes(bytes: &[u8]) -> Self {
        Self::Bytes(bytes.to_vec())
    }

    pub fn bigint(b_int: i64) -> Self {
        Self::BigInt(b_int)
    }

    pub fn decimal(dec: &str) -> Self {
        Self::Decimal(dec.to_owned())
    }

    pub fn scalar(val: impl Into<serde_json::Value>) -> Self {
        Self::Scalar(val.into())
    }
}

fn params_to_json(params: Vec<RawValue>) -> Vec<serde_json::Value> {
    params
        .into_iter()
        .map(|value| serde_json::Value::from(value))
        .collect::<Vec<_>>()
}

macro_rules! raw_value_from {
  ($($typ:ty),+) => {
      $(
          impl From<$typ> for RawValue {
              fn from(ty: $typ) -> Self {
                  Self::scalar(ty)
              }
          }
      )*
  };
}

raw_value_from!(String, &str, i32, i64, bool, f32, f64);

impl From<RawValue> for serde_json::Value {
    fn from(val: RawValue) -> Self {
        match val {
            RawValue::DateTime(dt) => scalar_type("datetime", dt.to_rfc3339()),
            RawValue::Bytes(bytes) => scalar_type("bytes", encode_bytes(&bytes)),
            RawValue::BigInt(b_int) => scalar_type("bigint", b_int.to_string()),
            RawValue::Decimal(dec) => scalar_type("decimal", dec.as_str()),
            RawValue::Scalar(v) => v,
        }
    }
}

fn scalar_type(type_name: &str, value: impl Into<serde_json::Value>) -> serde_json::Value {
    let value: serde_json::Value = value.into();

    serde_json::json!({ "prisma__type": type_name, "prisma__value": value })
}
