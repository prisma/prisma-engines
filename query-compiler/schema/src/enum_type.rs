use super::*;
use query_structure::{InternalEnum, PrismaValue, ScalarFieldRef};

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
    pub fn name(&self) -> String {
        self.identifier().name()
    }

    pub fn identifier(&self) -> &Identifier {
        match self {
            Self::String(s) => &s.identifier,
            Self::Database(db) => &db.identifier,
            Self::FieldRef(f) => &f.identifier,
        }
    }

    pub fn database(identifier: Identifier, internal_enum: InternalEnum) -> Self {
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
    identifier: Identifier,
    values: Vec<String>,
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
    identifier: Identifier,
    internal_enum: InternalEnum,
}

impl DatabaseEnumType {
    pub fn identifier(&self) -> &Identifier {
        &self.identifier
    }

    pub fn map_input_value(&self, val: &str) -> Option<PrismaValue> {
        Some(PrismaValue::Enum(
            self.internal_enum
                .walker()
                .values()
                .find(|ev| ev.database_name() == val)?
                .database_name()
                .to_owned(),
        ))
    }

    pub fn resolve_database_name(&self, val: &str) -> Option<&str> {
        self.internal_enum
            .walker()
            .values()
            .find(|ev| ev.name() == val)
            .map(|ev| ev.database_name())
    }

    pub fn external_values(&self) -> Vec<String> {
        self.internal_enum
            .walker()
            .values()
            .map(|v| v.name().to_owned())
            .collect()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct FieldRefEnumType {
    identifier: Identifier,
    values: Vec<(String, ScalarFieldRef)>,
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
