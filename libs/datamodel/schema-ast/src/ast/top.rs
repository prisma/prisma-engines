use crate::ast::{
    traits::WithSpan, CompositeType, Enum, Field, GeneratorConfig, Identifier, Model, SourceConfig, Span,
};

/// Enum for distinguishing between top-level entries
#[derive(Debug, Clone, PartialEq)]
pub enum Top {
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

    pub fn identifier(&self) -> &Identifier {
        match self {
            Top::CompositeType(ct) => &ct.name,
            Top::Enum(x) => &x.name,
            Top::Model(x) => &x.name,
            Top::Source(x) => &x.name,
            Top::Generator(x) => &x.name,
            Top::Type(x) => &x.name,
        }
    }

    pub fn name(&self) -> &str {
        &self.identifier().name
    }

    pub fn as_composite_type(&self) -> Option<&CompositeType> {
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

    pub fn as_enum(&self) -> Option<&Enum> {
        match self {
            Top::Enum(r#enum) => Some(r#enum),
            _ => None,
        }
    }

    pub fn as_generator(&self) -> Option<&GeneratorConfig> {
        match self {
            Top::Generator(gen) => Some(gen),
            _ => None,
        }
    }

    pub fn as_type_alias(&self) -> Option<&Field> {
        match self {
            Top::Type(r#type) => Some(r#type),
            _ => None,
        }
    }

    pub fn as_source(&self) -> Option<&SourceConfig> {
        match self {
            Top::Source(source) => Some(source),
            _ => None,
        }
    }
}

impl WithSpan for Top {
    fn span(&self) -> &Span {
        match self {
            Top::CompositeType(ct) => &ct.span,
            Top::Enum(en) => en.span(),
            Top::Model(model) => model.span(),
            Top::Source(source) => source.span(),
            Top::Generator(gen) => gen.span(),
            Top::Type(ty) => ty.span(),
        }
    }
}
