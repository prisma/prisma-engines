use quaint::{connector::ResultRowRef, prelude::ResultSet, Value};

pub trait ResultSetExt: Sized {
    fn assert_row_count(self, expected_count: usize) -> Self;

    fn assert_row(self, rowidx: usize, assertions: impl for<'a> FnOnce(RowAssertion<'a>) -> RowAssertion<'a>) -> Self;

    fn assert_first_row(self, assertions: impl for<'a> FnOnce(RowAssertion<'a>) -> RowAssertion<'a>) -> Self {
        self.assert_row(0, assertions)
    }

    fn assert_single_row(self, assertions: impl for<'a> FnOnce(RowAssertion<'a>) -> RowAssertion<'a>) -> Self {
        self.assert_row_count(1).assert_first_row(assertions)
    }
}

impl ResultSetExt for ResultSet {
    fn assert_row_count(self, expected_count: usize) -> Self {
        assert_eq!(self.len(), expected_count);

        self
    }

    fn assert_row(self, rowidx: usize, assertions: impl for<'a> FnOnce(RowAssertion<'a>) -> RowAssertion<'a>) -> Self {
        let assertion = RowAssertion(self.get(rowidx).unwrap());

        assertions(assertion);
        self
    }
}

#[derive(Debug)]
pub struct RowAssertion<'a>(ResultRowRef<'a>);

impl<'a> RowAssertion<'a> {
    pub fn assert_array_value(self, column_name: &str, expected_value: &[Value<'_>]) -> Self {
        let actual_value = self.0.get(column_name).and_then(|col: &Value<'_>| match col {
            Value::Array(x) => x.as_ref(),
            _ => panic!("as_array"),
        });

        assert_eq!(
            actual_value.map(|v| v.as_ref()),
            Some(expected_value),
            "Value assertion failed for {}. Expected: {:?}, got: {:?}",
            column_name,
            expected_value,
            actual_value,
        );

        self
    }

    pub fn assert_datetime_value(self, column_name: &str, expected_value: chrono::DateTime<chrono::Utc>) -> Self {
        let actual_value = self.0.get(column_name).and_then(|col: &Value<'_>| col.as_datetime());

        assert_eq!(
            actual_value,
            Some(expected_value),
            "Value assertion failed for {}. Expected: {:?}, got: {:?}",
            column_name,
            expected_value,
            actual_value,
        );

        self
    }

    #[allow(clippy::float_cmp)]
    pub fn assert_float_value(self, column_name: &str, expected_value: f64) -> Self {
        let actual_value = self
            .0
            .get(column_name)
            .and_then(|col: &Value<'_>| col.as_f64())
            .expect("Failed to extract f64");

        assert!(
            actual_value == expected_value,
            "Value assertion failed for {}. Expected: {:?}, got: {:?}",
            column_name,
            expected_value,
            actual_value,
        );

        self
    }

    pub fn assert_null_value(self, column_name: &str) -> Self {
        if !self.0.get(column_name).expect("not in result set").is_null() {
            panic!("Expected a null value for {}, but got something else.", column_name)
        }

        self
    }

    #[track_caller]
    pub fn assert_text_value(self, column_name: &str, expected_value: &str) -> Self {
        let value = self.0.get(column_name).expect("Expected a value, found none");
        let value_text: &str = match value {
            Value::Text(val) | Value::Enum(val) => val.as_deref(),
            _ => None,
        }
        .expect("Expected a string value");

        assert_eq!(
            value_text, expected_value,
            "Value assertion failed for {}. Expected: {:?}, got: {:?}",
            column_name, expected_value, value_text,
        );

        self
    }

    pub fn assert_int_value(self, column_name: &str, expected_value: i64) -> Self {
        let actual_value = self.0.get(column_name).and_then(|col: &Value<'_>| (*col).as_integer());

        assert!(
            actual_value == Some(expected_value),
            "Value assertion failed for {}. Expected: {:?}, got: {:?}",
            column_name,
            expected_value,
            actual_value,
        );

        self
    }
}
