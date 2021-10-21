use std::{cmp::Ordering, fmt, hash};

#[derive(Debug, Clone)]
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

impl PartialEq for Name {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Name::Model(left), Name::Model(right)) => left == right,
            (Name::Model(left), Name::CompositeType(right)) => left == right,
            (Name::CompositeType(left), Name::Model(right)) => left == right,
            (Name::CompositeType(left), Name::CompositeType(right)) => left == right,
        }
    }
}

impl Eq for Name {}

impl PartialOrd for Name {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        match (self, other) {
            (Name::Model(left), Name::Model(right)) => left.partial_cmp(right),
            (Name::Model(left), Name::CompositeType(right)) => left.partial_cmp(right),
            (Name::CompositeType(left), Name::Model(right)) => left.partial_cmp(right),
            (Name::CompositeType(left), Name::CompositeType(right)) => left.partial_cmp(right),
        }
    }
}

impl Ord for Name {
    fn cmp(&self, other: &Self) -> Ordering {
        match (self, other) {
            (Name::Model(left), Name::Model(right)) => left.cmp(right),
            (Name::Model(left), Name::CompositeType(right)) => left.cmp(right),
            (Name::CompositeType(left), Name::Model(right)) => left.cmp(right),
            (Name::CompositeType(left), Name::CompositeType(right)) => left.cmp(right),
        }
    }
}

impl hash::Hash for Name {
    fn hash<H: hash::Hasher>(&self, state: &mut H) {
        match self {
            Name::Model(name) => name.hash(state),
            Name::CompositeType(name) => name.hash(state),
        }
    }
}
