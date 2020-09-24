use prisma_models::{InternalEnum, ScalarFieldRef};

#[derive(Debug, Clone, PartialEq)]
pub enum EnumType {
    /// Generic, prisma-application specific string enum.
    /// Semantics are defined by the component interpreting the contents.
    String(StringEnumType),

    /// Enum from the internal data model, representing an enum on the database level.
    Internal(InternalEnum),

    /// Enum referencing fields on a model.
    FieldRef(FieldRefEnumType),
}

impl EnumType {
    pub fn name(&self) -> &str {
        match self {
            Self::String(s) => &s.name,
            Self::Internal(i) => &i.name,
            Self::FieldRef(f) => &f.name,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct StringEnumType {
    pub name: String,
    pub values: Vec<String>,
}

impl StringEnumType {
    /// Attempts to find an enum value for the given value key.
    pub fn value_for(&self, name: &str) -> Option<&str> {
        self.values
            .iter()
            .find_map(|val| if val == name { Some(val.as_str()) } else { None })
    }

    pub fn values(&self) -> &[String] {
        &self.values
    }
}

impl From<InternalEnum> for EnumType {
    fn from(internal_enum: InternalEnum) -> EnumType {
        EnumType::Internal(internal_enum)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct FieldRefEnumType {
    pub name: String,
    pub values: Vec<(String, ScalarFieldRef)>,
}

impl FieldRefEnumType {
    /// Attempts to find an enum value for the given value key.
    pub fn value_for(&self, name: &str) -> Option<&ScalarFieldRef> {
        self.values
            .iter()
            .find_map(|val| if &val.0 == name { Some(&val.1) } else { None })
    }

    pub fn values(&self) -> Vec<String> {
        self.values.iter().map(|(name, _)| name.to_owned()).collect()
    }
}
