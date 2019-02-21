use crate::ast::{Column, Row, Select};

#[derive(Debug, Clone, PartialEq)]
pub enum ParameterizedValue {
    Null,
    Integer(i64),
    Real(f64),
    Text(String),
    Boolean(bool),
}

#[derive(Debug, Clone, PartialEq)]
pub enum DatabaseValue {
    Parameterized(ParameterizedValue),
    Column(Column),
    Row(Row),
    Select(Select),
}

impl<'a> Into<ParameterizedValue> for &'a str {
    fn into(self) -> ParameterizedValue {
        ParameterizedValue::Text(self.to_string())
    }
}

impl Into<ParameterizedValue> for String {
    fn into(self) -> ParameterizedValue {
        ParameterizedValue::Text(self)
    }
}

impl Into<ParameterizedValue> for i64 {
    fn into(self) -> ParameterizedValue {
        ParameterizedValue::Integer(self)
    }
}

impl Into<ParameterizedValue> for f64 {
    fn into(self) -> ParameterizedValue {
        ParameterizedValue::Real(self)
    }
}

impl Into<ParameterizedValue> for bool {
    fn into(self) -> ParameterizedValue {
        ParameterizedValue::Boolean(self)
    }
}

impl<'a> Into<DatabaseValue> for &'a str {
    fn into(self) -> DatabaseValue {
        let val: ParameterizedValue = self.into();
        DatabaseValue::Parameterized(val)
    }
}

impl Into<DatabaseValue> for String {
    fn into(self) -> DatabaseValue {
        let val: ParameterizedValue = self.into();
        DatabaseValue::Parameterized(val)
    }
}

impl Into<DatabaseValue> for i64 {
    fn into(self) -> DatabaseValue {
        let val: ParameterizedValue = self.into();
        DatabaseValue::Parameterized(val)
    }
}

impl Into<DatabaseValue> for f64 {
    fn into(self) -> DatabaseValue {
        let val: ParameterizedValue = self.into();
        DatabaseValue::Parameterized(val)
    }
}

impl Into<DatabaseValue> for bool {
    fn into(self) -> DatabaseValue {
        let val: ParameterizedValue = self.into();
        DatabaseValue::Parameterized(val)
    }
}

impl Into<DatabaseValue> for ParameterizedValue {
    fn into(self) -> DatabaseValue {
        DatabaseValue::Parameterized(self)
    }
}

impl Into<DatabaseValue> for Row {
    fn into(self) -> DatabaseValue {
        DatabaseValue::Row(self)
    }
}
