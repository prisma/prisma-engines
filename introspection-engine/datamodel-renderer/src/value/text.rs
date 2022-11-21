use base64::display::Base64Display;
use psl::PreviewFeature;
use std::{borrow::Cow, fmt};

/// Represents a string value in the PSL.
#[derive(Debug, Clone, Copy)]
pub struct Text<T: fmt::Display>(pub(crate) T);

impl<'a> Text<Cow<'a, str>> {
    /// Construct a `Text` value from a string.
    pub fn new(s: impl Into<Cow<'a, str>>) -> Self {
        Text(s.into())
    }
}

impl<'a> fmt::Display for Text<&'a str> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&psl::schema_ast::string_literal(self.0), f)
    }
}

impl<'a> fmt::Display for Text<Cow<'a, str>> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&psl::schema_ast::string_literal(self.0.as_ref()), f)
    }
}

impl<'a> fmt::Display for Text<Base64Display<'a>> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("\"")?;
        self.0.fmt(f)?;
        f.write_str("\"")?;

        Ok(())
    }
}

impl fmt::Display for Text<PreviewFeature> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("\"")?;
        self.0.fmt(f)?;
        f.write_str("\"")?;

        Ok(())
    }
}
