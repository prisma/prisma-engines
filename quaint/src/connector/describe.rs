use std::borrow::Cow;

use super::ColumnType;

#[derive(Debug)]
pub struct DescribedQuery {
    pub parameters: Vec<DescribedParameter>,
    pub columns: Vec<DescribedColumn>,
    pub enum_names: Option<Vec<String>>,
}

impl DescribedQuery {
    pub fn param_enum_names(&self) -> Vec<&str> {
        self.parameters.iter().filter_map(|p| p.enum_name.as_deref()).collect()
    }
}

#[derive(Debug)]
pub struct DescribedParameter {
    pub name: String,
    pub typ: ColumnType,
    pub enum_name: Option<String>,
}

#[derive(Debug)]
pub struct DescribedColumn {
    pub name: String,
    pub typ: ColumnType,
    pub nullable: bool,
    pub enum_name: Option<String>,
}

impl DescribedParameter {
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

    pub fn set_typ(mut self, typ: ColumnType) -> Self {
        self.typ = typ;
        self
    }
}

impl DescribedColumn {
    pub fn new_named<'a>(name: impl Into<Cow<'a, str>>, typ: impl Into<ColumnType>) -> Self {
        let name: Cow<'_, str> = name.into();

        Self {
            name: name.into_owned(),
            typ: typ.into(),
            enum_name: None,
            nullable: false,
        }
    }

    pub fn new_unnamed(idx: usize, typ: impl Into<ColumnType>) -> Self {
        Self {
            name: format!("_{idx}"),
            typ: typ.into(),
            enum_name: None,
            nullable: false,
        }
    }

    pub fn with_enum_name(mut self, enum_name: Option<String>) -> Self {
        self.enum_name = enum_name;
        self
    }

    pub fn is_nullable(mut self, nullable: bool) -> Self {
        self.nullable = nullable;
        self
    }
}
