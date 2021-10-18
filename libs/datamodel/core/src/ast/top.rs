use super::{CompositeType, Enum, Field, GeneratorConfig, Identifier, Model, SourceConfig};

/// Enum for distinguishing between top-level entries
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Top {
    CompositeType(CompositeType),
    Enum(Enum),
    Model(Model),
    Source(SourceConfig),
    Generator(GeneratorConfig),
    Type(Field),
}

impl Top {
    pub fn get_type(&self) -> &str {
        match self {
            Top::CompositeType(_) => "composite type",
            Top::Enum(_) => "enum",
            Top::Model(_) => "model",
            Top::Source(_) => "source",
            Top::Generator(_) => "generator",
            Top::Type(_) => "type",
        }
    }

    pub(crate) fn identifier(&self) -> &Identifier {
        match self {
            Top::CompositeType(ct) => &ct.name,
            Top::Enum(x) => &x.name,
            Top::Model(x) => &x.name,
            Top::Source(x) => &x.name,
            Top::Generator(x) => &x.name,
            Top::Type(x) => &x.name,
        }
    }

    pub(crate) fn name(&self) -> &str {
        &self.identifier().name
    }

    pub(crate) fn as_composite_type(&self) -> Option<&CompositeType> {
        match self {
            Top::CompositeType(ct) => Some(ct),
            _ => None,
        }
    }

    pub fn as_model(&self) -> Option<&Model> {
        match self {
            Top::Model(model) => Some(model),
            _ => None,
        }
    }

    pub(crate) fn as_enum(&self) -> Option<&Enum> {
        match self {
            Top::Enum(r#enum) => Some(r#enum),
            _ => None,
        }
    }

    pub(crate) fn as_generator(&self) -> Option<&GeneratorConfig> {
        match self {
            Top::Generator(gen) => Some(gen),
            _ => None,
        }
    }

    pub(crate) fn as_type_alias(&self) -> Option<&Field> {
        match self {
            Top::Type(r#type) => Some(r#type),
            _ => None,
        }
    }

    pub(crate) fn as_source(&self) -> Option<&SourceConfig> {
        match self {
            Top::Source(source) => Some(source),
            _ => None,
        }
    }
}
