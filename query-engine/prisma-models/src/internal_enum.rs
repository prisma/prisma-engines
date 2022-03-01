use std::sync::Arc;

pub type InternalEnumRef = Arc<InternalEnum>;

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub struct InternalEnum {
    pub name: String,
    pub values: Vec<InternalEnumValue>,
}

impl InternalEnum {
    pub fn new<N, I, V>(name: N, values: I) -> Self
    where
        N: Into<String>,
        V: Into<InternalEnumValue>,
        I: IntoIterator<Item = V>,
    {
        Self {
            name: name.into(),
            values: values.into_iter().map(|v| v.into()).collect(),
        }
    }
}

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub struct InternalEnumValue {
    pub name: String,
    pub database_name: Option<String>,
}

impl InternalEnumValue {
    pub fn new<N, I, V>(name: N, database_name: I) -> Self
    where
        N: Into<String>,
        V: Into<String>,
        I: Into<Option<String>>,
    {
        Self {
            name: name.into(),
            database_name: database_name.into(),
        }
    }

    pub fn db_name(&self) -> &String {
        self.database_name.as_ref().unwrap_or(&self.name)
    }
}
