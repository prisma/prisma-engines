use crate::ast::{traits::WithSpan, CompositeType, Enum, GeneratorConfig, Identifier, Model, SourceConfig, Span};

use super::WithDocumentation;

/// Enum for distinguishing between top-level entries
#[derive(Debug, Clone)]
pub enum Top {
    /// A composite type
    CompositeType(CompositeType),
    /// An enum declaration
    Enum(Enum),
    /// A model declaration
    Model(Model),
    /// A datasource block
    Source(SourceConfig),
    /// A generator block
    Generator(GeneratorConfig),
}

impl Top {
    /// A string saying what kind of item this is.
    pub fn get_type(&self) -> &str {
        match self {
            Top::CompositeType(_) => "composite type",
            Top::Enum(_) => "enum",
            Top::Model(m) if m.is_view => "view",
            Top::Model(_) => "model",
            Top::Source(_) => "source",
            Top::Generator(_) => "generator",
        }
    }

    /// The name of the item.
    pub fn identifier(&self) -> &Identifier {
        match self {
            Top::CompositeType(ct) => &ct.name,
            Top::Enum(x) => &x.name,
            Top::Model(x) => &x.name,
            Top::Source(x) => &x.name,
            Top::Generator(x) => &x.name,
        }
    }

    /// The name of the item.
    pub fn name(&self) -> &str {
        &self.identifier().name
    }

    pub fn documentation(&self) -> Option<&str> {
        match self {
            Top::CompositeType(t) => t.documentation(),
            Top::Enum(t) => t.documentation(),
            Top::Model(t) => t.documentation(),
            Top::Source(t) => t.documentation(),
            Top::Generator(t) => t.documentation(),
        }
    }

    /// Try to interpret the item as a composite type declaration.
    pub fn as_composite_type(&self) -> Option<&CompositeType> {
        match self {
            Top::CompositeType(ct) => Some(ct),
            _ => None,
        }
    }

    /// Try to interpret the item as a model declaration.
    pub fn as_model(&self) -> Option<&Model> {
        match self {
            Top::Model(model) => Some(model),
            _ => None,
        }
    }

    /// Try to interpret the item as an enum declaration.
    pub fn as_enum(&self) -> Option<&Enum> {
        match self {
            Top::Enum(r#enum) => Some(r#enum),
            _ => None,
        }
    }

    /// Try to interpret the item as a generator block.
    pub fn as_generator(&self) -> Option<&GeneratorConfig> {
        match self {
            Top::Generator(gen) => Some(gen),
            _ => None,
        }
    }

    /// Try to interpret the item as a datasource block.
    pub fn as_source(&self) -> Option<&SourceConfig> {
        match self {
            Top::Source(source) => Some(source),
            _ => None,
        }
    }
}

impl WithSpan for Top {
    fn span(&self) -> Span {
        match self {
            Top::CompositeType(ct) => ct.span,
            Top::Enum(en) => en.span(),
            Top::Model(model) => model.span(),
            Top::Source(source) => source.span(),
            Top::Generator(gen) => gen.span(),
        }
    }
}
