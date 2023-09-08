use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum Name {
    Model(String),
    CompositeType(String),
}

impl Name {
    pub(super) fn as_model_name(&self) -> Option<&str> {
        match self {
            Name::Model(name) => Some(name),
            Name::CompositeType(_) => None,
        }
    }

    pub(super) fn as_type_name(&self) -> Option<&str> {
        match self {
            Name::Model(_) => None,
            Name::CompositeType(name) => Some(name),
        }
    }

    pub(crate) fn take(self) -> String {
        match self {
            Name::Model(name) => name,
            Name::CompositeType(name) => name,
        }
    }

    pub(super) fn is_composite_type(&self) -> bool {
        matches!(self, Self::CompositeType(_))
    }
}

impl fmt::Display for Name {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Name::Model(name) => f.write_str(name),
            Name::CompositeType(name) => f.write_str(name),
        }
    }
}

impl AsRef<str> for Name {
    fn as_ref(&self) -> &str {
        match self {
            Name::Model(name) => name.as_ref(),
            Name::CompositeType(name) => name.as_ref(),
        }
    }
}
