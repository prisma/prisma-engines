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

    pub fn inner(&self) -> &str {
        self.0.as_ref()
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

impl AsRef<str> for EnumVariant<'_> {
    fn as_ref(&self) -> &str {
        self.0.as_ref()
    }
}

impl std::ops::Deref for EnumVariant<'_> {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl fmt::Display for EnumVariant<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_ref())
    }
}

impl<'a> From<Cow<'a, str>> for EnumVariant<'a> {
    fn from(value: Cow<'a, str>) -> Self {
        Self(value)
    }
}

impl From<String> for EnumVariant<'_> {
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
