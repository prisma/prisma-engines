use std::borrow::Cow;

use super::ColumnType;

pub struct ParsedRawQuery {
    pub parameters: Vec<ParsedRawItem>,
    pub columns: Vec<ParsedRawItem>,
}

pub struct ParsedRawItem {
    pub name: String,
    pub typ: ColumnType,
    pub enum_name: Option<String>,
}

impl ParsedRawItem {
    pub fn new_named<'a>(name: impl Into<Cow<'a, str>>, typ: impl Into<ColumnType>) -> Self {
        let name: Cow<'_, str> = name.into();

        Self {
            name: name.into_owned(),
            typ: typ.into(),
            enum_name: None,
        }
    }

    pub fn new_unnamed(idx: usize, typ: impl Into<ColumnType>) -> Self {
        Self {
            name: format!("_{idx}"),
            typ: typ.into(),
            enum_name: None,
        }
    }

    pub fn with_enum_name(mut self, enum_name: Option<String>) -> Self {
        self.enum_name = enum_name;
        self
    }
}
