use chrono::{DateTime, FixedOffset};
use prisma_value::encode_bytes;
use query_tests_setup::{TestError, TestResult};

pub fn fmt_query_raw(query: &str, params: impl IntoIterator<Item = RawParam>) -> String {
    let params = params_to_json(params);
    let params = serde_json::to_string(&params).unwrap();

    format!(
        r#"mutation {{ queryRaw(query: "{}", parameters: "{}") }}"#,
        query.replace('"', "\\\"").replace('\n', ""),
        params.replace('"', "\\\"")
    )
}

pub fn fmt_execute_raw(query: &str, params: impl IntoIterator<Item = RawParam>) -> String {
    let params = params_to_json(params);
    let params = serde_json::to_string(&params).unwrap();

    format!(
        r#"mutation {{ executeRaw(query: "{}", parameters: "{}") }}"#,
        query.replace('"', "\\\"").replace('\n', ""),
        params.replace('"', "\\\"")
    )
}

/// Small abstraction over query raw parameters.
/// Useful to differentiate floats from decimals as they're encoded differently in clients.
pub enum RawParam {
    DateTime(DateTime<FixedOffset>),
    Bytes(Vec<u8>),
    BigInt(i64),
    Decimal(String),
    Array(Vec<RawParam>),
    Primitive(serde_json::Value),
    Null,
}

impl RawParam {
    pub fn try_datetime(s: &str) -> TestResult<Self> {
        let datetime = DateTime::parse_from_rfc3339(s).map_err(|err| TestError::ParseError(err.to_string()))?;

        Ok(Self::DateTime(datetime))
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

    pub fn array(arr: Vec<impl Into<RawParam>>) -> Self {
        let arr: Vec<_> = arr.into_iter().map(Into::into).collect();

        Self::Array(arr)
    }

    pub fn scalar(val: impl Into<serde_json::Value>) -> Self {
        Self::Primitive(val.into())
    }
}

fn params_to_json(params: impl IntoIterator<Item = RawParam>) -> Vec<serde_json::Value> {
    params.into_iter().map(serde_json::Value::from).collect::<Vec<_>>()
}

macro_rules! raw_value_from {
  ($($typ:ty),+) => {
      $(
          impl From<$typ> for RawParam {
              fn from(ty: $typ) -> Self {
                  Self::scalar(ty)
              }
          }
      )*
  };
}

raw_value_from!(String, &str, i64, bool, f64);

impl From<RawParam> for serde_json::Value {
    fn from(val: RawParam) -> Self {
        match val {
            RawParam::DateTime(dt) => scalar_type("date", dt.to_rfc3339()),
            RawParam::Bytes(bytes) => scalar_type("bytes", encode_bytes(&bytes)),
            RawParam::BigInt(b_int) => scalar_type("bigint", b_int.to_string()),
            RawParam::Decimal(dec) => scalar_type("decimal", dec.as_str()),
            RawParam::Array(values) => {
                let json_values: Vec<_> = values.into_iter().map(serde_json::Value::from).collect();

                serde_json::Value::Array(json_values)
            }
            RawParam::Primitive(v) => v,
            RawParam::Null => serde_json::Value::Null,
        }
    }
}

fn scalar_type(type_name: &str, value: impl Into<serde_json::Value>) -> serde_json::Value {
    let value: serde_json::Value = value.into();

    serde_json::json!({ "prisma__type": type_name, "prisma__value": value })
}
