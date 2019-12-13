use super::*;

/// Enum for distinguishing between top-level entries
#[derive(Debug, Clone, PartialEq)]
pub enum Top {
    Enum(Enum),
    Model(Model),
    Source(SourceConfig),
    Generator(GeneratorConfig),
    Type(Field),
}

impl WithIdentifier for Top {
    fn identifier(&self) -> &Identifier {
        match self {
            Top::Enum(x) => x.identifier(),
            Top::Model(x) => x.identifier(),
            Top::Source(x) => x.identifier(),
            Top::Generator(x) => x.identifier(),
            Top::Type(x) => x.identifier(),
        }
    }
}

impl WithSpan for Top {
    fn span(&self) -> &Span {
        match self {
            Top::Enum(x) => x.span(),
            Top::Model(x) => x.span(),
            Top::Source(x) => x.span(),
            Top::Generator(x) => x.span(),
            Top::Type(x) => x.span(),
        }
    }
}

impl Top {
    pub fn get_type(&self) -> &str {
        match self {
            Top::Enum(_) => "enum",
            Top::Model(_) => "model",
            Top::Source(_) => "source",
            Top::Generator(_) => "generator",
            Top::Type(_) => "type",
        }
    }

    pub fn name(&self) -> &str {
        match self {
            Top::Enum(x) => &x.name.name,
            Top::Model(x) => &x.name.name,
            Top::Source(x) => &x.name.name,
            Top::Generator(x) => &x.name.name,
            Top::Type(x) => &x.name.name,
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
