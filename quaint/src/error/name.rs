#[derive(Debug, PartialEq, Eq)]
pub enum Name {
    Available(String),
    Unavailable,
}

impl Name {
    pub fn available(name: impl ToString) -> Self {
        Self::Available(name.to_string())
    }
}

impl std::fmt::Display for Name {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Available(name) => name.fmt(f),
            Self::Unavailable => write!(f, "(not available)"),
        }
    }
}

impl<T> From<Option<T>> for Name
where
    T: ToString,
{
    fn from(name: Option<T>) -> Self {
        match name {
            Some(name) => Self::available(name),
            None => Self::Unavailable,
        }
    }
}
