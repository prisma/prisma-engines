use std::{borrow::Cow, fmt};

/// A unquoted identifier. Should be used if a value has certain naming standards.
#[derive(Debug)]
pub struct Constant<T: fmt::Display>(pub(crate) T);

impl Clone for Constant<&str> {
    fn clone(&self) -> Self {
        *self
    }
}

impl Clone for Constant<Cow<'_, str>> {
    fn clone(&self) -> Self {
        Constant(self.0.clone())
    }
}

impl Copy for Constant<&str> {}

impl AsRef<str> for Constant<&str> {
    fn as_ref(&self) -> &str {
        self.0
    }
}

impl AsRef<str> for Constant<Cow<'_, str>> {
    fn as_ref(&self) -> &str {
        self.0.as_ref()
    }
}

impl<T> Constant<T>
where
    T: fmt::Display,
{
    pub(crate) fn new_no_validate(value: T) -> Self {
        Self(value)
    }

    pub(crate) fn into_inner(self) -> T {
        self.0
    }
}

impl<T> fmt::Display for Constant<T>
where
    T: fmt::Display,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl<T> From<T> for Constant<T>
where
    T: fmt::Display,
{
    fn from(inner: T) -> Self {
        Self(inner)
    }
}
