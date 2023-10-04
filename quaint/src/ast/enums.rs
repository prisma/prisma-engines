use crate::Value;
use std::{borrow::Cow, fmt};

#[derive(Debug, Clone, PartialEq)]
pub struct EnumVariant<'a>(Cow<'a, str>);

impl<'a> EnumVariant<'a> {
    pub fn new(variant: impl Into<Cow<'a, str>>) -> Self {
        Self(variant.into())
    }

    pub fn into_owned(self) -> String {
        self.0.into_owned()
    }

    pub fn into_text(self) -> Value<'a> {
        Value::text(self.0)
    }

    pub fn into_enum(self, name: Option<EnumName<'a>>) -> Value<'a> {
        match name {
            Some(name) => Value::enum_variant_with_name(self.0, name),
            None => Value::enum_variant(self.0),
        }
    }
}

impl<'a> AsRef<str> for EnumVariant<'a> {
    fn as_ref(&self) -> &str {
        self.0.as_ref()
    }
}

impl<'a> std::ops::Deref for EnumVariant<'a> {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'a> fmt::Display for EnumVariant<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_ref())
    }
}

impl<'a> From<Cow<'a, str>> for EnumVariant<'a> {
    fn from(value: Cow<'a, str>) -> Self {
        Self(value)
    }
}

impl<'a> From<String> for EnumVariant<'a> {
    fn from(value: String) -> Self {
        Self(value.into())
    }
}

impl<'a> From<&'a str> for EnumVariant<'a> {
    fn from(value: &'a str) -> Self {
        Self(value.into())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct EnumName<'a> {
    pub name: Cow<'a, str>,
    pub schema_name: Option<Cow<'a, str>>,
}

impl<'a> EnumName<'a> {
    pub fn new(name: impl Into<Cow<'a, str>>, schema_name: Option<impl Into<Cow<'a, str>>>) -> Self {
        Self {
            name: name.into(),
            schema_name: schema_name.map(|s| s.into()),
        }
    }
}
