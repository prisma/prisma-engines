use super::*;
use prisma_models::{InternalEnumRef, PrismaValue, ScalarFieldRef};

#[derive(Debug, Clone, PartialEq)]
pub enum EnumType {
    /// Generic, prisma-application specific string enum.
    /// Semantics are defined by the component interpreting the contents.
    String(StringEnumType),

    /// Enum from the internal data model, representing an enum on the database level.
    Database(DatabaseEnumType),

    /// Enum referencing fields on a model.
    FieldRef(FieldRefEnumType),
}

impl EnumType {
    pub fn name(&self) -> &str {
        match self {
            Self::String(s) => &s.name,
            Self::Database(db) => &db.name,
            Self::FieldRef(f) => &f.name,
        }
    }

    // Used as cache keys, for example.
    pub fn identifier(&self) -> Identifier {
        Identifier::new(self.name().to_owned(), self.namespace())
    }

    pub fn namespace(&self) -> String {
        match self {
            Self::String(_) => PRISMA_NAMESPACE,
            Self::Database(_) => MODEL_NAMESPACE,
            Self::FieldRef(_) => PRISMA_NAMESPACE,
        }
        .to_string()
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
        self.values.iter().find_map(|val| (val == name).then(|| val.as_str()))
    }

    pub fn values(&self) -> &[String] {
        &self.values
    }
}

impl From<InternalEnumRef> for EnumType {
    fn from(internal_enum: InternalEnumRef) -> EnumType {
        EnumType::Database(DatabaseEnumType {
            name: internal_enum.name.clone(),
            internal_enum,
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct DatabaseEnumType {
    pub name: String,
    pub internal_enum: InternalEnumRef,
}

impl DatabaseEnumType {
    pub fn map_input_value(&self, val: &str) -> Option<PrismaValue> {
        Some(PrismaValue::Enum(
            self.internal_enum
                .values
                .iter()
                .find(|ev| ev.name == val)?
                .db_name()
                .clone(),
        ))
    }

    pub fn map_output_value(&self, val: &str) -> Option<PrismaValue> {
        Some(PrismaValue::Enum(
            self.internal_enum
                .values
                .iter()
                .find(|ev| ev.db_name() == val)?
                .name
                .clone(),
        ))
    }

    pub fn external_values(&self) -> Vec<String> {
        self.internal_enum
            .values
            .iter()
            .map(|v| v.name.to_string())
            .collect::<Vec<String>>()
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
        self.values.iter().find_map(|val| (val.0 == name).then(|| &val.1))
    }

    pub fn values(&self) -> Vec<String> {
        self.values.iter().map(|(name, _)| name.to_owned()).collect()
    }
}

impl From<EnumType> for OutputType {
    fn from(e: EnumType) -> Self {
        OutputType::Enum(Arc::new(e))
    }
}

impl From<EnumType> for InputType {
    fn from(e: EnumType) -> Self {
        InputType::Enum(Arc::new(e))
    }
}
