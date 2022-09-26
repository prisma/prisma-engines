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
        self.identifier().name()
    }

    pub fn identifier(&self) -> &Identifier {
        match self {
            Self::String(s) => &s.identifier,
            Self::Database(db) => &db.identifier,
            Self::FieldRef(f) => &f.identifier,
        }
    }

    pub fn database(identifier: Identifier, internal_enum: InternalEnumRef) -> Self {
        Self::Database(DatabaseEnumType {
            identifier,
            internal_enum,
        })
    }

    pub fn field_ref(identifier: Identifier, values: Vec<(String, ScalarFieldRef)>) -> Self {
        Self::FieldRef(FieldRefEnumType { identifier, values })
    }

    pub fn string(identifier: Identifier, values: Vec<String>) -> Self {
        Self::String(StringEnumType { identifier, values })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct StringEnumType {
    pub identifier: Identifier,
    pub values: Vec<String>,
}

impl StringEnumType {
    pub fn identifier(&self) -> &Identifier {
        &self.identifier
    }

    /// Attempts to find an enum value for the given value key.
    pub fn value_for(&self, name: &str) -> Option<&str> {
        self.values.iter().find_map(|val| (val == name).then_some(val.as_str()))
    }

    pub fn values(&self) -> &[String] {
        &self.values
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct DatabaseEnumType {
    pub identifier: Identifier,
    pub internal_enum: InternalEnumRef,
}

impl DatabaseEnumType {
    pub fn identifier(&self) -> &Identifier {
        &self.identifier
    }

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
    pub identifier: Identifier,
    pub values: Vec<(String, ScalarFieldRef)>,
}

impl FieldRefEnumType {
    pub fn identifier(&self) -> &Identifier {
        &self.identifier
    }

    /// Attempts to find an enum value for the given value key.
    pub fn value_for(&self, name: &str) -> Option<&ScalarFieldRef> {
        self.values.iter().find_map(|val| (val.0 == name).then_some(&val.1))
    }

    pub fn values(&self) -> Vec<String> {
        self.values.iter().map(|(name, _)| name.to_owned()).collect()
    }
}
