use crate::AssertionResult;
use quaint::{connector::ResultRowRef, prelude::ResultSet, Value};

pub trait ResultSetExt: Sized {
    fn assert_row_count(self, expected_count: usize) -> AssertionResult<Self>;

    fn assert_row(
        self,
        rowidx: usize,
        assertions: impl for<'a> FnOnce(RowAssertion<'a>) -> AssertionResult<RowAssertion<'a>>,
    ) -> AssertionResult<Self>;

    fn assert_first_row(
        self,
        assertions: impl for<'a> FnOnce(RowAssertion<'a>) -> AssertionResult<RowAssertion<'a>>,
    ) -> AssertionResult<Self> {
        self.assert_row(0, assertions)
    }

    fn assert_single_row(
        self,
        assertions: impl for<'a> FnOnce(RowAssertion<'a>) -> AssertionResult<RowAssertion<'a>>,
    ) -> AssertionResult<Self> {
        self.assert_row_count(1)?.assert_first_row(assertions)
    }
}

impl ResultSetExt for ResultSet {
    fn assert_row_count(self, expected_count: usize) -> AssertionResult<Self> {
        assert_eq!(self.len(), expected_count);

        Ok(self)
    }

    fn assert_row(
        self,
        rowidx: usize,
        assertions: impl for<'a> FnOnce(RowAssertion<'a>) -> AssertionResult<RowAssertion<'a>>,
    ) -> AssertionResult<Self> {
        let assertion = RowAssertion(self.get(rowidx).ok_or_else(|| anyhow::anyhow!("TODO"))?);

        assertions(assertion)?;

        Ok(self)
    }
}

#[derive(Debug)]
pub struct RowAssertion<'a>(ResultRowRef<'a>);

impl<'a> RowAssertion<'a> {
    pub fn assert_array_value(self, column_name: &str, expected_value: &[Value<'_>]) -> AssertionResult<Self> {
        let actual_value = self.0.get(column_name).and_then(|col: &Value<'_>| match col {
            Value::Array(x) => x.as_ref(),
            _ => panic!("as_array"),
        });

        anyhow::ensure!(
            actual_value.map(|v| v.as_ref()) == Some(expected_value),
            "Value assertion failed for {}. Expected: {:?}, got: {:?}",
            column_name,
            expected_value,
            actual_value,
        );

        Ok(self)
    }

    pub fn assert_datetime_value(
        self,
        column_name: &str,
        expected_value: chrono::DateTime<chrono::Utc>,
    ) -> AssertionResult<Self> {
        let actual_value = self.0.get(column_name).and_then(|col: &Value<'_>| col.as_datetime());

        anyhow::ensure!(
            actual_value == Some(expected_value),
            "Value assertion failed for {}. Expected: {:?}, got: {:?}",
            column_name,
            expected_value,
            actual_value,
        );

        Ok(self)
    }

    pub fn assert_float_value(self, column_name: &str, expected_value: f64) -> AssertionResult<Self> {
        let actual_value = self.0.get(column_name).and_then(|col: &Value<'_>| col.as_f64());

        anyhow::ensure!(
            actual_value == Some(expected_value),
            "Value assertion failed for {}. Expected: {:?}, got: {:?}",
            column_name,
            expected_value,
            actual_value,
        );

        Ok(self)
    }

    pub fn assert_null_value(self, column_name: &str) -> AssertionResult<Self> {
        if !self.0.get(column_name).expect("not in result set").is_null() {
            anyhow::bail!("Expected a null value for {}, but got something else.", column_name)
        }

        Ok(self)
    }

    pub fn assert_text_value(self, column_name: &str, expected_value: &str) -> AssertionResult<Self> {
        let actual_value = self.0.get(column_name).and_then(|col: &Value<'_>| (*col).to_string());

        anyhow::ensure!(
            actual_value.as_deref() == Some(expected_value),
            "Value assertion failed for {}. Expected: {:?}, got: {:?}",
            column_name,
            expected_value,
            actual_value,
        );

        Ok(self)
    }

    pub fn assert_int_value(self, column_name: &str, expected_value: i64) -> AssertionResult<Self> {
        let actual_value = self.0.get(column_name).and_then(|col: &Value<'_>| (*col).as_i64());

        anyhow::ensure!(
            actual_value == Some(expected_value),
            "Value assertion failed for {}. Expected: {:?}, got: {:?}",
            column_name,
            expected_value,
            actual_value,
        );

        Ok(self)
    }
}
